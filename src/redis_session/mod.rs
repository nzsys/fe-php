use anyhow::{Context, Result};
use redis::{aio::ConnectionManager, AsyncCommands, Client};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, error};

/// Redis session manager for distributed session storage
pub struct RedisSessionManager {
    client: Client,
    connection_manager: ConnectionManager,
    key_prefix: String,
    default_ttl: Duration,
}

impl RedisSessionManager {
    /// Create a new Redis session manager
    pub async fn new(url: &str, key_prefix: String, timeout_ms: u64) -> Result<Self> {
        let client = Client::open(url).context("Failed to create Redis client")?;

        let connection_manager = ConnectionManager::new(client.clone())
            .await
            .context("Failed to connect to Redis")?;

        debug!("Connected to Redis at {}", url);

        Ok(Self {
            client,
            connection_manager,
            key_prefix,
            default_ttl: Duration::from_millis(timeout_ms),
        })
    }

    /// Generate a full Redis key with prefix
    fn make_key(&self, session_id: &str) -> String {
        format!("{}{}", self.key_prefix, session_id)
    }

    /// Store a session
    pub async fn set_session<T: Serialize>(
        &mut self,
        session_id: &str,
        data: &T,
        ttl: Option<Duration>,
    ) -> Result<()> {
        let key = self.make_key(session_id);
        let value = serde_json::to_string(data).context("Failed to serialize session data")?;
        let ttl_seconds = ttl.unwrap_or(self.default_ttl).as_secs();

        self.connection_manager
            .set_ex(&key, value, ttl_seconds as u64)
            .await
            .context("Failed to set session in Redis")?;

        debug!("Stored session {} with TTL {} seconds", session_id, ttl_seconds);
        Ok(())
    }

    /// Retrieve a session
    pub async fn get_session<T: for<'de> Deserialize<'de>>(
        &mut self,
        session_id: &str,
    ) -> Result<Option<T>> {
        let key = self.make_key(session_id);

        let value: Option<String> = self
            .connection_manager
            .get(&key)
            .await
            .context("Failed to get session from Redis")?;

        match value {
            Some(v) => {
                let data: T = serde_json::from_str(&v)
                    .context("Failed to deserialize session data")?;
                debug!("Retrieved session {}", session_id);
                Ok(Some(data))
            }
            None => {
                debug!("Session {} not found", session_id);
                Ok(None)
            }
        }
    }

    /// Delete a session
    pub async fn delete_session(&mut self, session_id: &str) -> Result<()> {
        let key = self.make_key(session_id);

        self.connection_manager
            .del::<_, ()>(&key)
            .await
            .context("Failed to delete session from Redis")?;

        debug!("Deleted session {}", session_id);
        Ok(())
    }

    /// Check if a session exists
    pub async fn exists_session(&mut self, session_id: &str) -> Result<bool> {
        let key = self.make_key(session_id);

        let exists: bool = self
            .connection_manager
            .exists(&key)
            .await
            .context("Failed to check session existence in Redis")?;

        Ok(exists)
    }

    /// Extend the TTL of a session
    pub async fn refresh_session(&mut self, session_id: &str, ttl: Option<Duration>) -> Result<()> {
        let key = self.make_key(session_id);
        let ttl_seconds = ttl.unwrap_or(self.default_ttl).as_secs();

        self.connection_manager
            .expire::<_, ()>(&key, ttl_seconds as i64)
            .await
            .context("Failed to refresh session TTL in Redis")?;

        debug!("Refreshed session {} with TTL {} seconds", session_id, ttl_seconds);
        Ok(())
    }

    /// Get all session keys (for debugging/admin purposes)
    pub async fn get_all_sessions(&mut self) -> Result<Vec<String>> {
        let pattern = format!("{}*", self.key_prefix);

        let keys: Vec<String> = self
            .connection_manager
            .keys(pattern)
            .await
            .context("Failed to get session keys from Redis")?;

        Ok(keys
            .into_iter()
            .filter_map(|k| k.strip_prefix(&self.key_prefix).map(|s| s.to_string()))
            .collect())
    }

    /// Clear all sessions (use with caution!)
    pub async fn clear_all_sessions(&mut self) -> Result<()> {
        let pattern = format!("{}*", self.key_prefix);

        let keys: Vec<String> = self
            .connection_manager
            .keys(&pattern)
            .await
            .context("Failed to get session keys from Redis")?;

        if !keys.is_empty() {
            self.connection_manager
                .del::<_, ()>(&keys)
                .await
                .context("Failed to delete sessions from Redis")?;

            debug!("Cleared {} sessions", keys.len());
        }

        Ok(())
    }

    /// Ping Redis to check connection
    pub async fn ping(&mut self) -> Result<()> {
        redis::cmd("PING")
            .query_async::<_, ()>(&mut self.connection_manager)
            .await
            .context("Failed to ping Redis")?;
        Ok(())
    }
}

/// Default session data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub user_id: Option<String>,
    pub created_at: i64,
    pub last_accessed: i64,
    pub data: serde_json::Value,
}

impl SessionData {
    pub fn new() -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            user_id: None,
            created_at: now,
            last_accessed: now,
            data: serde_json::json!({}),
        }
    }

    pub fn touch(&mut self) {
        self.last_accessed = chrono::Utc::now().timestamp();
    }
}

impl Default for SessionData {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_redis_session_manager_requires_redis() {
        // This test would need a running Redis instance
        // In a real scenario, you would use a test Redis instance or mock
    }
}
