use anyhow::{Context, Result};
use redis::Client;
use crate::config::RedisConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct User {
    pub username: String,
    pub email: String,
    pub team_name: Option<String>,
}

impl User {
    pub fn new(username: String, email: String) -> Self {
        Self {
            username,
            email,
            team_name: None,
        }
    }
    
    pub fn with_team(username: String, email: String, team_name: String) -> Self {
        Self {
            username,
            email,
            team_name: Some(team_name),
        }
    }
    
    pub fn with_team_and_id(username: String, email: String, team_name: String, user_id: u64) -> Self {
        Self {
            username,
            email,
            team_name: Some(team_name)
        }
    }
    
    // Convert user to Redis storage format (username:email)
    pub fn to_redis_format(&self) -> String {
        format!("{}:{}", self.username, self.email)
    }
    
    // Parse user from Redis storage format (username:email)
    pub fn from_redis_format(data: &str) -> Option<Self> {
        let parts: Vec<&str> = data.split(':').collect();
        if parts.len() == 2 {
            Some(Self::new(parts[0].to_string(), parts[1].to_string()))
        } else {
            None
        }
    }
}

#[derive(Clone)]   
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
    pub fn get_team_score_by_check(&self, competition_name: &str, team_name: &str, check_name: &str, check_points : i64) -> Result<i64> {
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
    //returns an array of team score events for a given check for a given time range
    pub fn get_team_score_check_events(&self, competition_name: &str, team_name: &str, check_name: &str, time_start : i64, time_end: i64) -> Result<Vec<(String, String)>> {
        let mut conn = self.client.get_connection().context("Failed to connect to Redis")?;
        
        // the key name
        let key = format!("{}:{}:{}", competition_name, team_name, check_name);
        
        // Get the events for this team in this competition
        let events: Vec<String> = redis::cmd("ZRANGEBYSCORE")
            .arg(&key)
            .arg(time_start)
            .arg(time_end)
            .arg("WITHSCORES")
            .query(&mut conn)
            .context("Failed to get team score check events")?;
        
        // Convert to Vec<(String, String)>. Note that WITHSCORES returns a Vec of alternating values, so we need to pair them up
        let mut result = Vec::new();
        for chunk in events.chunks(2) {
            if chunk.len() == 2 {
                result.push((chunk[0].clone(), chunk[1].clone()));
            }
        }
        Ok(result)
    }

    // Write SSH keypair for a box. Returns true if written, false if key exists.
    pub fn write_ssh_keypair(&self, competition_name: &str, team_name: &str, box_name: &str, private_key: &str) -> Result<bool> {
        let mut conn = self.client.get_connection().context("Failed to connect to Redis")?;
        let key = format!("{}:{}:{}:ssh_keypair", competition_name, team_name, box_name);
        // NX: Only set if not exists
        let res: Option<String> = redis::cmd("SET")
            .arg(&key)
            .arg(private_key)
            .arg("NX")
            .query(&mut conn)
            .context("Failed to write SSH keypair")?;
        Ok(res.is_some())
    }

    // Read SSH keypair for a box. Returns None if not found.
    pub fn read_ssh_keypair(&self, competition_name: &str, team_name: &str, box_name: &str) -> Result<Option<String>> {
        let mut conn = self.client.get_connection().context("Failed to connect to Redis")?;
        let key = format!("{}:{}:{}:ssh_keypair", competition_name, team_name, box_name);
        let val: Option<String> = redis::cmd("GET")
            .arg(&key)
            .query(&mut conn)
            .context("Failed to read SSH keypair")?;
        Ok(val)
    }

    // Write username/password for a box. Returns true if written, false if key exists.
    pub fn write_box_credentials(&self, competition_name: &str, team_name: &str, box_name: &str, username: &str, password: &str) -> Result<bool> {
        let mut conn = self.client.get_connection().context("Failed to connect to Redis")?;
        let key = format!("{}:{}:{}:credentials", competition_name, team_name, box_name);
        let value = format!("{}:{}", username, password);
        let res: Option<String> = redis::cmd("SET")
            .arg(&key)
            .arg(value)
            .arg("NX")
            .query(&mut conn)
            .context("Failed to write box credentials")?;
        Ok(res.is_some())
    }

    // Read username/password for a box. Returns None if not found.
    pub fn read_box_credentials(&self, competition_name: &str, team_name: &str, box_name: &str) -> Result<Option<(String, String)>> {
        let mut conn = self.client.get_connection().context("Failed to connect to Redis")?;
        let key = format!("{}:{}:{}:credentials", competition_name, team_name, box_name);
        let val: Option<String> = redis::cmd("GET")
            .arg(&key)
            .query(&mut conn)
            .context("Failed to read box credentials")?;
        if let Some(s) = val {
            let mut parts = s.splitn(2, ':');
            if let (Some(username), Some(password)) = (parts.next(), parts.next()) {
                return Ok(Some((username.to_string(), password.to_string())));
            }
        }
        Ok(None)
    }

    // Register a user to a team. Creates/inserts a new key in competition_name:users and competition_name:team_name:users
    // competition_name:users -> set of usernames, emails.
    // takes a User struct and team_name
    // if user already exists, register them to the new team, check iteratively which team they are registered to, and remove them from the old team.
    pub fn register_user(
        &self,
        competition_name: &str,
        user: &User,
        team_name: &str,
    ) -> Result<()> {
        let mut conn = self.client.get_connection().context("Failed to connect to Redis")?;
        
        // Keys for Redis operations
        let users_key = format!("{}:users", competition_name);
        let user_data = user.to_redis_format();
        let new_team_users_key = format!("{}:{}:users", competition_name, team_name);
        
        // Check if user already exists in the competition
        let user_exists: bool = redis::cmd("SISMEMBER")
            .arg(&users_key)
            .arg(&user_data)
            .query(&mut conn)
            .context("Failed to check if user exists")?;
        
        if user_exists {
            // User exists, need to find their current team and move them
            // Get all teams for this competition to check which one the user is in
            let pattern = format!("{}:*:users", competition_name);
            let team_keys: Vec<String> = redis::cmd("KEYS")
                .arg(&pattern)
                .query(&mut conn)
                .context("Failed to get team keys")?;
            
            // Find the team the user is currently in
            for team_key in team_keys {
                let is_member: bool = redis::cmd("SISMEMBER")
                    .arg(&team_key)
                    .arg(&user_data)
                    .query(&mut conn)
                    .context("Failed to check team membership")?;
                
                if is_member {
                    // Remove user from old team
                    let _: () = redis::cmd("SREM")
                        .arg(&team_key)
                        .arg(&user_data)
                        .query(&mut conn)
                        .context("Failed to remove user from old team")?;
                    break;
                }
            }
        } else {
            // New user, add to global users set
            let _: () = redis::cmd("SADD")
                .arg(&users_key)
                .arg(&user_data)
                .query(&mut conn)
                .context("Failed to add user to global users set")?;
        }
        
        // Add user to the new team
        let _: () = redis::cmd("SADD")
            .arg(&new_team_users_key)
            .arg(&user_data)
            .query(&mut conn)
            .context("Failed to add user to new team")?;
        
        Ok(())
    }
    
    // Get all users for a team
    pub fn get_team_users(&self, competition_name: &str, team_name: &str) -> Result<Vec<User>> {
        let mut conn = self.client.get_connection().context("Failed to connect to Redis")?;
        
        let team_users_key = format!("{}:{}:users", competition_name, team_name);
        let users: Vec<String> = redis::cmd("SMEMBERS")
            .arg(&team_users_key)
            .query(&mut conn)
            .context("Failed to get team users")?;
        
        let result = users.into_iter()
            .filter_map(|user_data| {
                let mut user = User::from_redis_format(&user_data)?;
                user.team_name = Some(team_name.to_string());
                Some(user)
            })
            .collect();
        
        Ok(result)
    }

    // Store an oauth2 pkce_verifier.
    // key should expire in 5 minutes.
    pub fn store_pkce_verifier(&self, verifier: &str) -> Result<()> {
        let mut conn = self.client.get_connection().context("Failed to connect to Redis")?;
        
        let _: () = redis::cmd("SADD")
            .arg("pkce_verifiers")
            .arg(verifier)
            .query(&mut conn)
            .context("Failed to store PKCE challenge")?;
        Ok(())
    }

    pub fn get_pkce_verifier(&self, verifier: &str) -> Result<Option<String>> {
        let mut conn = self.client.get_connection().context("Failed to connect to Redis")?;
        
        // Get the verifier for the given challenge. If it doesn't exist, return None.
        let verifier: Option<String> = redis::cmd("SISMEMBER")
            .arg("pkce_verifiers")
            .arg(verifier)
            .query(&mut conn)
            .context("Failed to check PKCE verifier existence")?;
        
        Ok(verifier)
    }
    
    // Get all users in the competition
    pub fn get_all_users(&self, competition_name: &str) -> Result<Vec<User>> {
        let mut conn = self.client.get_connection().context("Failed to connect to Redis")?;
        
        let users_key = format!("{}:users", competition_name);
        let users: Vec<String> = redis::cmd("SMEMBERS")
            .arg(&users_key)
            .query(&mut conn)
            .context("Failed to get all users")?;
        
        let result = users.into_iter()
            .filter_map(|user_data| User::from_redis_format(&user_data))
            .collect();
        
        Ok(result)
    }

    // Get a specific user by username and find their team
    pub fn get_user(&self, competition_name: &str, username: &str) -> Result<Option<User>> {
        let mut conn = self.client.get_connection().context("Failed to connect to Redis")?;
        
        // Get all users to find the one with matching username
        let users_key = format!("{}:users", competition_name);
        let users: Vec<String> = redis::cmd("SMEMBERS")
            .arg(&users_key)
            .query(&mut conn)
            .context("Failed to get all users")?;
        
        // Find user with matching username
        let user_data = users.into_iter()
            .find(|data| data.starts_with(&format!("{}:", username)));
        
        if let Some(data) = user_data {
            if let Some(mut user) = User::from_redis_format(&data) {
                // Find which team this user belongs to
                let pattern = format!("{}:*:users", competition_name);
                let team_keys: Vec<String> = redis::cmd("KEYS")
                    .arg(&pattern)
                    .query(&mut conn)
                    .context("Failed to get team keys")?;
                
                for team_key in team_keys {
                    let is_member: bool = redis::cmd("SISMEMBER")
                        .arg(&team_key)
                        .arg(&data)
                        .query(&mut conn)
                        .context("Failed to check team membership")?;
                    
                    if is_member {
                        // Extract team name from the key (format: competition:team:users)
                        let parts: Vec<&str> = team_key.split(':').collect();
                        if parts.len() >= 2 {
                            user.team_name = Some(parts[parts.len() - 2].to_string());
                        }
                        break;
                    }
                }
                
                return Ok(Some(user));
            }
        }
        
        Ok(None)
    }
}
