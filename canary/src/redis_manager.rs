use anyhow::{Context, Result};
use redis::Client;
use carve::config::RedisConfig;

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
    
    pub fn record_sucessful_check_result(
        &self,
        competition_name: &str,
        check_name: &str,
        timestamp_ms: i64,
        team_name: &str,
        box_name: &str,
        message: &str,
    ) -> Result<String> {
        let mut conn = self.client.get_connection().context("Failed to connect to Redis")?;
        
        // the key name
        let key = format!("{}:{}:{}", competition_name, team_name, check_name);
        // insert the score to the sorted set if it does not exist, with the timestamp as the score
        let _: () = redis::cmd("ZADD")
            .arg(&key)
            .arg(timestamp_ms)
            .arg(format!("{}:{}", box_name, message))
            .query(&mut conn)
            .context("Failed to record successful check result")?;
        // Return the key name for confirmation
        Ok(key)



        
    }
    
    
    
    // Get detailed teams scores by check
    pub fn get_team_score_check(&self, competition_name: &str, team_name: &str, check_name: &str, check_points : i64) -> Result<i64> {
        let mut conn = self.client.get_connection().context("Failed to connect to Redis")?;
        
        // the key name
        let key = format!("{}:{}:{}", competition_name, team_name, check_name);
        
        // Get the total score for this team in this competition
        let score: i64 = redis::cmd("ZCARD")
            .arg(&key)
            .query(&mut conn)
            .context("Failed to get team score")?;

        // multiply by the number of points per check
        let score = score * check_points;
        
        Ok(score)
    }
}
