use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::redis::RedisClient;

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
            data: serde_json::Value::Object(serde_json::Map::new()),
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

/// Session manager with Redis backend
pub struct SessionManager {
    redis: Arc<RedisClient>,
}

impl SessionManager {
    pub fn new(redis: Arc<RedisClient>) -> Self {
        Self { redis }
    }

    /// Create a new session
    pub async fn create_session(&self) -> Result<String> {
        let session_id = Uuid::new_v4().to_string();
        let session_data = SessionData::new();
        let serialized = serde_json::to_string(&session_data)?;

        self.redis.set_session(&session_id, &serialized).await?;

        Ok(session_id)
    }

    /// Get session data
    pub async fn get_session(&self, session_id: &str) -> Result<Option<SessionData>> {
        if let Some(data) = self.redis.get_session(session_id).await? {
            let session_data: SessionData = serde_json::from_str(&data)?;
            Ok(Some(session_data))
        } else {
            Ok(None)
        }
    }

    /// Update session data
    pub async fn update_session(&self, session_id: &str, mut session_data: SessionData) -> Result<()> {
        session_data.touch();
        let serialized = serde_json::to_string(&session_data)?;
        self.redis.set_session(session_id, &serialized).await?;
        Ok(())
    }

    /// Delete session
    pub async fn delete_session(&self, session_id: &str) -> Result<()> {
        self.redis.delete_session(session_id).await
    }

    /// Check if session exists
    pub async fn exists(&self, session_id: &str) -> Result<bool> {
        self.redis.exists(session_id).await
    }

    /// Extend session TTL
    pub async fn extend_session(&self, session_id: &str) -> Result<()> {
        self.redis.extend_session(session_id).await
    }

    /// Get all active sessions (admin use)
    pub async fn get_all_sessions(&self) -> Result<Vec<String>> {
        self.redis.get_all_sessions("*").await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_data_creation() {
        let session = SessionData::new();
        assert!(session.user_id.is_none());
        assert!(session.created_at > 0);
        assert_eq!(session.created_at, session.last_accessed);
    }

    #[test]
    fn test_session_touch() {
        let mut session = SessionData::new();
        let original_time = session.last_accessed;

        std::thread::sleep(std::time::Duration::from_millis(10));
        session.touch();

        assert!(session.last_accessed > original_time);
    }
}
