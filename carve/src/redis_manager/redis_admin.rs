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
}
