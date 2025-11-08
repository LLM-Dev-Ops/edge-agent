//! Mock cache implementations for testing

use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use crate::helpers::*;

/// Mock L1 cache (in-memory)
pub struct MockL1Cache {
    storage: Arc<RwLock<HashMap<String, CacheEntry>>>,
    hit_count: Arc<std::sync::atomic::AtomicU64>,
    miss_count: Arc<std::sync::atomic::AtomicU64>,
}

struct CacheEntry {
    response: ChatCompletionResponse,
    expires_at: Option<std::time::Instant>,
}

impl MockL1Cache {
    pub fn new() -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
            hit_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            miss_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    pub async fn get(&self, key: &str) -> Option<ChatCompletionResponse> {
        let storage = self.storage.read().await;
        if let Some(entry) = storage.get(key) {
            if let Some(expires_at) = entry.expires_at {
                if std::time::Instant::now() > expires_at {
                    drop(storage);
                    self.remove(key).await;
                    self.miss_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    return None;
                }
            }
            self.hit_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Some(entry.response.clone())
        } else {
            self.miss_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            None
        }
    }

    pub async fn set(&self, key: String, response: ChatCompletionResponse, ttl: Option<std::time::Duration>) {
        let expires_at = ttl.map(|d| std::time::Instant::now() + d);
        let entry = CacheEntry {
            response,
            expires_at,
        };
        self.storage.write().await.insert(key, entry);
    }

    pub async fn remove(&self, key: &str) {
        self.storage.write().await.remove(key);
    }

    pub async fn clear(&self) {
        self.storage.write().await.clear();
    }

    pub fn hits(&self) -> u64 {
        self.hit_count.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn misses(&self) -> u64 {
        self.miss_count.load(std::sync::atomic::Ordering::Relaxed)
    }
}

/// Mock L2 cache (Redis simulation)
pub struct MockL2Cache {
    storage: Arc<RwLock<HashMap<String, CacheEntry>>>,
    failure_mode: Arc<std::sync::atomic::AtomicBool>,
    latency: Arc<RwLock<std::time::Duration>>,
}

impl MockL2Cache {
    pub fn new() -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
            failure_mode: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            latency: Arc::new(RwLock::new(std::time::Duration::from_millis(2))),
        }
    }

    pub async fn get(&self, key: &str) -> Result<Option<ChatCompletionResponse>, CacheError> {
        if self.failure_mode.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(CacheError::ConnectionFailed);
        }

        // Simulate network latency
        let latency = *self.latency.read().await;
        tokio::time::sleep(latency).await;

        let storage = self.storage.read().await;
        if let Some(entry) = storage.get(key) {
            if let Some(expires_at) = entry.expires_at {
                if std::time::Instant::now() > expires_at {
                    drop(storage);
                    self.remove(key).await?;
                    return Ok(None);
                }
            }
            Ok(Some(entry.response.clone()))
        } else {
            Ok(None)
        }
    }

    pub async fn set(&self, key: String, response: ChatCompletionResponse, ttl: Option<std::time::Duration>) -> Result<(), CacheError> {
        if self.failure_mode.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(CacheError::ConnectionFailed);
        }

        let latency = *self.latency.read().await;
        tokio::time::sleep(latency).await;

        let expires_at = ttl.map(|d| std::time::Instant::now() + d);
        let entry = CacheEntry {
            response,
            expires_at,
        };
        self.storage.write().await.insert(key, entry);
        Ok(())
    }

    pub async fn remove(&self, key: &str) -> Result<(), CacheError> {
        if self.failure_mode.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(CacheError::ConnectionFailed);
        }

        self.storage.write().await.remove(key);
        Ok(())
    }

    pub async fn clear(&self) -> Result<(), CacheError> {
        if self.failure_mode.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(CacheError::ConnectionFailed);
        }

        self.storage.write().await.clear();
        Ok(())
    }

    pub fn set_failure_mode(&self, enabled: bool) {
        self.failure_mode.store(enabled, std::sync::atomic::Ordering::Relaxed);
    }

    pub async fn set_latency(&self, latency: std::time::Duration) {
        *self.latency.write().await = latency;
    }
}

/// Cache error types
#[derive(Debug, Clone)]
pub enum CacheError {
    ConnectionFailed,
    Timeout,
    SerializationError,
    Unknown(String),
}

impl std::fmt::Display for CacheError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConnectionFailed => write!(f, "Connection failed"),
            Self::Timeout => write!(f, "Timeout"),
            Self::SerializationError => write!(f, "Serialization error"),
            Self::Unknown(msg) => write!(f, "Unknown error: {}", msg),
        }
    }
}

impl std::error::Error for CacheError {}

/// Mock Redis client for testing
pub struct MockRedis {
    inner: MockL2Cache,
}

impl MockRedis {
    pub fn new() -> Self {
        Self {
            inner: MockL2Cache::new(),
        }
    }

    pub async fn connect(url: &str) -> Result<Self, CacheError> {
        Ok(Self::new())
    }

    pub fn simulate_failure(&self, enabled: bool) {
        self.inner.set_failure_mode(enabled);
    }

    pub async fn ping(&self) -> Result<(), CacheError> {
        if self.inner.failure_mode.load(std::sync::atomic::Ordering::Relaxed) {
            Err(CacheError::ConnectionFailed)
        } else {
            Ok(())
        }
    }
}
