use crate::config;

use super::*;

impl RedisManager {
    // Generate a new API key and store it in Redis
    pub async fn generate_api_key(&self) -> Result<String> {
        let api_key = Self::generate_hex_string(16);
        self.redis_sadd("carve:api_keys", &api_key).await?;
        Ok(api_key)
    }

    // Remove an API key from Redis
    pub async fn remove_api_key(&self, api_key: &str) -> Result<()> {
        let mut conn = self.get_connection().await?;
        redis::cmd("SREM")
            .arg("carve:api_keys")
            .arg(api_key)
            .query_async(&mut conn)
            .await
            .context("Failed to remove API key")
    }

    // Check if an API key exists in Redis
    pub async fn check_api_key_exists(&self, api_key: &str) -> Result<bool> {
        let mut conn = self.get_connection().await?;
        let exists: bool = redis::cmd("SISMEMBER")
            .arg("carve:api_keys")
            .arg(api_key)
            .query_async(&mut conn)
            .await
            .context("Failed to check API key existence")?;
        Ok(exists)
    }

    // get api keys list
    pub async fn get_api_keys(&self) -> Result<Vec<String>> {
        let mut conn = self.get_connection().await?;
        redis::cmd("SMEMBERS")
            .arg("carve:api_keys")
            .query_async(&mut conn)
            .await
            .context("Failed to get API keys")
    }

    pub async fn publish_toast(&self, toast: &config::ToastNotification) -> Result<()> {
        let mut conn = self.get_connection().await?;
        match serde_yaml::to_string(toast) {
            Ok(payload) => {
                if let Some(ref user) = toast.user {
                    redis::cmd("PUBLISH")
                        .arg(format!("carve:toasts:user:{}", user))
                        .arg(payload.clone())
                        .query_async::<()>(&mut conn)
                        .await
                        .context("Failed to publish user-specific toast notification")?;
                }
                else if let Some(ref team) = toast.team {
                    redis::cmd("PUBLISH")
                        .arg(format!("carve:toasts:team:{}", team))
                        .arg(payload)
                        .query_async::<()>(&mut conn)
                        .await
                        .context("Failed to publish team-specific toast notification")?;
                } else {
                    redis::cmd("PUBLISH")
                        .arg("carve:toasts")
                        .arg(payload)
                        .query_async::<()>(&mut conn)
                        .await
                        .context("Failed to publish toast notification")?;
                }
            }
            Err(e) => return Err(anyhow::anyhow!("Failed to serialize toast notification: {}", e)),
        }
        Ok(())
    }

    pub async fn wait_for_next_toast(&self, user: Option<String>, team: Option<String>) -> Result<Option<config::ToastNotification>> {
        let (mut sink, mut stream) = self
            .client
            .get_async_pubsub()
            .await
            .context("Failed to get Redis pubsub connection")?
            .split();
        sink.subscribe("carve:toasts")
            .await
            .context("Failed to subscribe to toast notifications")?;
        if let Some(user) = user {
            sink.subscribe(&format!("carve:toasts:user:{}", user))
                .await
                .context("Failed to subscribe to user-specific toast notifications")?;
        }
        if let Some(team) = team {
            sink.subscribe(&format!("carve:toasts:team:{}", team))
                .await
                .context("Failed to subscribe to team-specific toast notifications")?;
        }
        let msg = stream
            .next()
            .await;
        if let Some(msg) = msg {
            if let Ok(toast) = serde_yaml::from_str::<config::ToastNotification>(&msg.get_payload::<String>()?) {
                Ok(Some(toast))
            } else {    
                Err(anyhow::anyhow!("Failed to deserialize toast notification"))
            }
        } else {
            Ok(None)
        }
    }
}
