use futures_util::future::BoxFuture;
use opentelemetry::{global, trace::TracerProvider};
use opentelemetry_sdk::error::OTelSdkResult;
use opentelemetry_sdk::trace::{SdkTracerProvider, SpanData, SpanExporter};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use tracing_opentelemetry::OpenTelemetryLayer;

#[derive(Debug, Clone)]
struct StdoutExporter;

impl SpanExporter for StdoutExporter {
    fn export(&self, batch: Vec<SpanData>) -> BoxFuture<'static, OTelSdkResult> {
        Box::pin(async move {
            for span in batch {
                println!(
                    "OTEL TRACE: name={} status={:?} attributes={:?}",
                    span.name, span.status, span.attributes
                );
            }

            Ok(())
        })
    }
}

pub fn init_tracing() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let provider = SdkTracerProvider::builder()
        .with_simple_exporter(StdoutExporter)
        .build();

    global::set_tracer_provider(provider.clone());
    let tracer = provider.tracer("ai-gateway");

    let otel_layer = OpenTelemetryLayer::new(tracer);

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .with(otel_layer)
        .try_init()?;

    Ok(())
}
