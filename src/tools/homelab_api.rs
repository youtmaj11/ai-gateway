use reqwest::Client;
use serde_json::Value;
use std::env;
use std::fmt;

use crate::tools::Tool;

#[derive(Debug)]
pub enum HomelabApiError {
    Request(String),
    Configuration(String),
}

impl fmt::Display for HomelabApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HomelabApiError::Request(err) => write!(f, "request failed: {err}"),
            HomelabApiError::Configuration(err) => write!(f, "configuration error: {err}"),
        }
    }
}

impl std::error::Error for HomelabApiError {}

pub struct HomelabApiTool;

impl HomelabApiTool {
    fn homelab_url() -> Result<String, HomelabApiError> {
        env::var("AI_GATEWAY_HOMELAB_URL").map_err(|_| {
            HomelabApiError::Configuration("AI_GATEWAY_HOMELAB_URL is required".to_string())
        })
    }

    fn jwt_token() -> Result<String, HomelabApiError> {
        env::var("AI_GATEWAY_HOMELAB_JWT").map_err(|_| {
            HomelabApiError::Configuration("AI_GATEWAY_HOMELAB_JWT is required".to_string())
        })
    }

    fn parse_params(params: &str) -> Result<(String, String), HomelabApiError> {
        let trimmed = params.trim();

        let mut pieces = trimmed.split_whitespace();
        match pieces.next() {
            Some("argocd") => match pieces.next() {
                Some("sync_status") => {
                    if let Some(app_name) = pieces.next() {
                        Ok(("argocd".to_string(), format!("/api/v1/applications/{app_name}")))
                    } else {
                        Err(HomelabApiError::Configuration(
                            "argocd sync_status requires application name".to_string(),
                        ))
                    }
                }
                _ => Err(HomelabApiError::Configuration(
                    "Unsupported argocd action, expected: argocd sync_status <app>".to_string(),
                )),
            },
            Some("grafana") => match pieces.next() {
                Some("health") => Ok(("grafana".to_string(), "/api/health".to_string())),
                _ => Err(HomelabApiError::Configuration(
                    "Unsupported grafana action, expected: grafana health".to_string(),
                )),
            },
            Some(path) if path.starts_with('/') => Ok(("generic".to_string(), path.to_string())),
            _ => Err(HomelabApiError::Configuration(
                "Unsupported homelab endpoint. Use argocd sync_status <app> or grafana health".to_string(),
            )),
        }
    }

    async fn call_endpoint(service: &str, path: &str) -> Result<String, HomelabApiError> {
        let base_url = Self::homelab_url()?;
        let token = Self::jwt_token()?;
        let client = Client::builder().build().map_err(|err| HomelabApiError::Request(err.to_string()))?;

        let url = if service == "generic" {
            format!("{}{}", base_url.trim_end_matches('/'), path)
        } else {
            format!("{}{}", base_url.trim_end_matches('/'), path)
        };

        let resp = client
            .get(&url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|err| HomelabApiError::Request(err.to_string()))?;

        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|err| HomelabApiError::Request(err.to_string()))?;

        if status.is_success() {
            if let Ok(json_value) = serde_json::from_str::<Value>(&body) {
                Ok(format!(
                    "Homelab API response ({}):\n{}",
                    status,
                    serde_json::to_string_pretty(&json_value)
                        .unwrap_or_else(|_| body.clone())
                ))
            } else {
                Ok(format!("Homelab API response ({}):\n{}", status, body))
            }
        } else {
            Err(HomelabApiError::Request(format!(
                "{} returned status {}: {}",
                url, status, body
            )))
        }
    }
}

impl Tool for HomelabApiTool {
    fn name(&self) -> &'static str {
        "homelab_api"
    }

    fn execute(&self, params: &str) -> String {
        let result = match Self::parse_params(params) {
            Ok((service, path)) => tokio::runtime::Handle::current().block_on(Self::call_endpoint(&service, &path)),
            Err(err) => Err(err),
        };

        match result {
            Ok(output) => output,
            Err(err) => format!("HomelabApiTool error: {err}"),
        }
    }
}
