use crate::storage::redis::RedisCache;

/// Agent core helper functions for the CLI and future runtime.
pub async fn run_chat(message: &str, mut cache: Option<&mut RedisCache>) -> String {
    let key = format!("chat:{}", message);

    if let Some(cache) = cache.as_mut() {
        if let Ok(Some(cached)) = cache.get(&key).await {
            return format!("Cached response: {}", cached);
        }
    }

    let response = format!("Agent stub response: {}", message);

    if let Some(cache) = cache.as_mut() {
        let _ = cache.set(&key, &response, 60).await;
    }

    response
}
