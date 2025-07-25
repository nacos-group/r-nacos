use std::time::Duration;
use tokio::time::timeout;

pub async fn wait_for<F, Fut>(f: F, max_duration: Duration) -> bool
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    timeout(max_duration, async {
        loop {
            if f().await {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .unwrap_or(false)
}

pub fn generate_unique_id(prefix: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();

    let counter = COUNTER.fetch_add(1, Ordering::SeqCst);

    format!("{}_{}_{}", prefix, timestamp, counter)
}

pub fn generate_test_data_id() -> String {
    generate_unique_id("test_data_id")
}

pub fn generate_test_group() -> String {
    generate_unique_id("test_group")
}

pub fn generate_test_service() -> String {
    generate_unique_id("test_service")
}

pub fn generate_test_namespace() -> String {
    generate_unique_id("test_namespace")
}
