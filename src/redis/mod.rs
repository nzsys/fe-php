use anyhow::Result;
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Redis client wrapper for session storage
pub struct RedisClient {
    manager: Arc<RwLock<ConnectionManager>>,
    ttl: u64,
}

impl RedisClient {
    /// Create a new Redis client
    pub async fn new(url: &str, ttl: u64) -> Result<Self> {
        let client = redis::Client::open(url)?;
        let manager = ConnectionManager::new(client).await?;

        Ok(Self {
            manager: Arc::new(RwLock::new(manager)),
            ttl,
        })
    }

    /// Store a session
    pub async fn set_session(&self, session_id: &str, data: &str) -> Result<()> {
        let mut conn = self.manager.write().await;
        conn.set_ex::<_, _, ()>(session_id, data, self.ttl).await?;
        Ok(())
    }

    /// Retrieve a session
    pub async fn get_session(&self, session_id: &str) -> Result<Option<String>> {
        let mut conn = self.manager.write().await;
        let result: Option<String> = conn.get(session_id).await?;
        Ok(result)
    }

    /// Delete a session
    pub async fn delete_session(&self, session_id: &str) -> Result<()> {
        let mut conn = self.manager.write().await;
        conn.del::<_, ()>(session_id).await?;
        Ok(())
    }

    /// Check if a session exists
    pub async fn exists(&self, session_id: &str) -> Result<bool> {
        let mut conn = self.manager.write().await;
        let result: bool = conn.exists(session_id).await?;
        Ok(result)
    }

    /// Extend session TTL
    pub async fn extend_session(&self, session_id: &str) -> Result<()> {
        let mut conn = self.manager.write().await;
        conn.expire::<_, ()>(session_id, self.ttl as i64).await?;
        Ok(())
    }

    /// Get all sessions (for debugging/admin purposes)
    pub async fn get_all_sessions(&self, pattern: &str) -> Result<Vec<String>> {
        let mut conn = self.manager.write().await;
        let keys: Vec<String> = conn.keys(pattern).await?;
        Ok(keys)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires Redis to be running
    async fn test_redis_session() {
        let client = RedisClient::new("redis://127.0.0.1:6379", 60)
            .await
            .expect("Failed to connect to Redis");

        client
            .set_session("test_session", "test_data")
            .await
            .expect("Failed to set session");

        let data = client
            .get_session("test_session")
            .await
            .expect("Failed to get session");

        assert_eq!(data, Some("test_data".to_string()));

        client
            .delete_session("test_session")
            .await
            .expect("Failed to delete session");

        let data = client
            .get_session("test_session")
            .await
            .expect("Failed to get session");

        assert_eq!(data, None);
    }
}
