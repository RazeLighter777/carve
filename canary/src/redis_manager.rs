use anyhow::{Context, Result};
use redis::Client;
use crate::config::RedisConfig;

pub struct RedisManager {
    client: Client,
}

impl RedisManager {
    pub fn new(config: &RedisConfig) -> Result<Self> {
        let redis_url = format!("redis://{}:{}/{}", config.host, config.port, config.db);
        let client = Client::open(redis_url).context("Failed to create Redis client")?;
        
        Ok(Self { client })
    }
    
    pub fn health_check(&self) -> Result<()> {
        let mut conn = self.client.get_connection().context("Failed to connect to Redis")?;
        redis::cmd("PING").query::<String>(&mut conn).context("Failed to ping Redis")?;
        Ok(())
    }
    
    pub fn record_check_result(
        &self,
        competition_name: &str,
        check_name: &str,
        timestamp_ms: i64,
        success: bool,
        team_name: &str,
        box_name: &str,
        message: &str,
    ) -> Result<String> {
        let mut conn = self.client.get_connection().context("Failed to connect to Redis")?;
        
        // Create a unique ID with timestamp-*
        let stream_key = format!("{}:{}", competition_name, check_name);
        let result_value = if success { "1" } else { "0" };
        
        // XADD stream_key timestamp-* result 1|0 team "team-name" box "box-name" message "message"
        let id: String = redis::cmd("XADD")
            .arg(&stream_key)
            .arg(format!("{}-*", timestamp_ms))
            .arg("result")
            .arg(result_value)
            .arg("team")
            .arg(team_name)
            .arg("box")
            .arg(box_name)
            .arg("message")
            .arg(message)
            .query(&mut conn)
            .context("Failed to add entry to Redis stream")?;
        
        Ok(id)
    }
}
