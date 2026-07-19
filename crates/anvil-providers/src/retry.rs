// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

use anvil_core::{AnvilError, Result};
use std::future::Future;
use tracing::warn;

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay_ms: 500,
            max_delay_ms: 30_000,
        }
    }
}

fn is_retryable(err: &AnvilError) -> bool {
    if let AnvilError::Provider(msg) = err {
        let lower = msg.to_lowercase();
        lower.contains("429")
            || lower.contains("rate limit")
            || lower.contains("too many requests")
            || lower.contains("503")
    } else {
        false
    }
}

pub async fn with_retry<F, Fut, T>(config: &RetryConfig, f: F) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let mut attempt = 0u32;
    loop {
        match f().await {
            Ok(v) => return Ok(v),
            Err(e) if attempt + 1 < config.max_attempts && is_retryable(&e) => {
                let delay = (config.base_delay_ms * (1u64 << attempt)).min(config.max_delay_ms);
                warn!(
                    "provider request failed (attempt {}/{}): {}; retrying in {}ms",
                    attempt + 1,
                    config.max_attempts,
                    e,
                    delay
                );
                tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
                attempt += 1;
            }
            Err(e) => return Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn succeeds_on_first_attempt() {
        let calls = Arc::new(AtomicU32::new(0));
        let c = Arc::clone(&calls);
        let result = with_retry(
            &RetryConfig {
                max_attempts: 3,
                base_delay_ms: 1,
                max_delay_ms: 10,
            },
            || {
                let c = Arc::clone(&c);
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    Ok::<_, AnvilError>(42u32)
                }
            },
        )
        .await;
        assert_eq!(result.unwrap(), 42);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn retries_on_rate_limit_then_succeeds() {
        let calls = Arc::new(AtomicU32::new(0));
        let c = Arc::clone(&calls);
        let result = with_retry(
            &RetryConfig {
                max_attempts: 3,
                base_delay_ms: 1,
                max_delay_ms: 10,
            },
            || {
                let c = Arc::clone(&c);
                async move {
                    let n = c.fetch_add(1, Ordering::SeqCst);
                    if n < 2 {
                        Err(AnvilError::Provider(
                            "HTTP 429 Too Many Requests".to_string(),
                        ))
                    } else {
                        Ok::<_, AnvilError>(99u32)
                    }
                }
            },
        )
        .await;
        assert_eq!(result.unwrap(), 99);
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn does_not_retry_non_retryable_error() {
        let calls = Arc::new(AtomicU32::new(0));
        let c = Arc::clone(&calls);
        let result = with_retry::<_, _, ()>(
            &RetryConfig {
                max_attempts: 3,
                base_delay_ms: 1,
                max_delay_ms: 10,
            },
            || {
                let c = Arc::clone(&c);
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    Err(AnvilError::Provider("401 Unauthorized".to_string()))
                }
            },
        )
        .await;
        assert!(result.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn exhausts_retries_on_persistent_rate_limit() {
        let calls = Arc::new(AtomicU32::new(0));
        let c = Arc::clone(&calls);
        let cfg = RetryConfig {
            max_attempts: 3,
            base_delay_ms: 1,
            max_delay_ms: 10,
        };
        let result = with_retry::<_, _, ()>(&cfg, || {
            let c = Arc::clone(&c);
            async move {
                c.fetch_add(1, Ordering::SeqCst);
                Err(AnvilError::Provider("rate limit exceeded".to_string()))
            }
        })
        .await;
        assert!(result.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }
}
