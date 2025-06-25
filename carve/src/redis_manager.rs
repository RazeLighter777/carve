use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rand::distr::Distribution;
use redis::Client;
use crate::config::RedisConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct User {
    pub username: String,
    pub email: String,
    pub team_name: Option<String>,
    pub is_admin: bool, // Optional field to indicate if the user is an admin]
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CompetitionStatus {
    Active,
    Unstarted,
    Finished,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
/// Commands for managing QEMU instances
pub enum QemuCommands {
    Start,
    Stop,
    Restart,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScoreEvent {
    pub message: String,
    pub timestamp: DateTime<chrono::Utc>,
    pub team_id: u64,
    pub score_event_type: String, // e.g., "icmp_check_1"
    pub box_name: String, // Name of the box where the event occurred
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompetitionState {
    pub name: String,
    pub status: CompetitionStatus,
    pub start_time: Option<DateTime<Utc>>, // Unix timestamp in seconds
    pub end_time: Option<DateTime<Utc>>,   // Unix timestamp in seconds
}


impl User {
    pub fn new(username: String, email: String) -> Self {
        Self {
            username,
            email,
            team_name: None,
            is_admin: false, // Default to false, can be set later
        }
    }
    
    pub fn with_team(username: String, email: String, team_name: String) -> Self {
        Self {
            username,
            email,
            team_name: Some(team_name),
            is_admin: false, // Default to false, can be set later
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
    
    pub fn generate_team_join_code(&self, competition_name: &str, team_name: &str) -> Result<u64> {
        let mut conn = self.client.get_connection().context("Failed to connect to Redis")?;
        
        // Generate a unique join code (u64) 
        let join_code: u64 = rand::random::<u64>() % 1_000_000_000; // 9-digit code
        // Store the team name with the join code
        let key = format!("{}:team_join_codes", competition_name);
        let _: () = redis::cmd("HSET")
            .arg(&key)
            .arg(join_code)
            .arg(team_name)
            .query(&mut conn)
            .context("Failed to store team join code")?;
        // set an expiration time for the join code (optional, e.g., 24 hours)
        let _: () = redis::cmd("HEXPIRE")
            .arg(&key)
            .arg(86400) // 24 hours in seconds
            .arg("FIELDS")
            .arg(1)
            .arg(join_code)
            .query(&mut conn)
            .context("Failed to set expiration for team join code")?;
        Ok(join_code)
    }
    // Generates a box console code for a team. This is a unique code that can be used to access the team's boxes,
    // and is passed to novnc proxy in the url path.
    // This is a 32 character alphanumeric code.
    // if the code already exists, it will return the existing code.
    pub fn get_box_console_code(&self, competition_name: &str, team_name: &str) -> Result<String> {
        let mut conn = self.client.get_connection().context("Failed to connect to Redis")?;
        // Check if the console code already exists
        let key = format!("{}:box_console_codes", competition_name);
        if let Some(console_code) = redis::cmd("HGET")
            .arg(&key)
            .arg(team_name)
            .query(&mut conn)
            .context("Failed to get box console code")?
        {
            return Ok(console_code);
        }
        // Generate a new console code
        let console_code: String = rand::distr::Alphanumeric
            .sample_iter(&mut rand::rng())
            .take(32)
            .map(char::from)
            .collect();
        let _: () = redis::cmd("HSET")
            .arg(&key)
            .arg(team_name)
            .arg(&console_code)
            .query(&mut conn)
            .context("Failed to store box console code")?;
        
        Ok(console_code)
    }

    pub fn check_team_join_code(&self, competition_name: &str, join_code: u64) -> Result<Option<String>> {
        let mut conn = self.client.get_connection().context("Failed to connect to Redis")?;
        
        // Check if the join code exists
        let key = format!("{}:team_join_codes", competition_name);
        let team_name: Option<String> = redis::cmd("HGET")
            .arg(&key)
            .arg(join_code)
            .query(&mut conn)
            .context("Failed to check team join code")?;
        
        Ok(team_name)
    }


    pub fn health_check(&self) -> Result<()> {
        let mut conn = self.client.get_connection().context("Failed to connect to Redis")?;
        redis::cmd("PING").query::<String>(&mut conn).context("Failed to ping Redis")?;
        Ok(())
    }

    // wait for events for qemu boxes.
    // this blocking call takes an iterator of events, and waits one event to happen.
    pub fn wait_for_qemu_event(&self, competition_name: &str, team_name: &str, box_name: &str, events : impl Iterator<Item = QemuCommands>) -> Result<QemuCommands> {
        let mut conn = self.client.get_connection().context("Failed to connect to Redis")?;
        
        // the key name
        let key = format!("{}:{}:{}:events", competition_name, team_name, box_name);
        
        // Subscribe to the key for events
        let mut pubsub = conn.as_pubsub();
        pubsub.subscribe(&key).context("Failed to subscribe to Redis channel")?;
        let mut result =         Err(anyhow::anyhow!("No valid QEMU command received"));

        // Wait for an event
        let msg = pubsub.get_message().context("Failed to get message from Redis")?;
        let payload: String = msg.get_payload().context("Failed to get payload from message")?;
        if let Ok(command) = serde_yaml::from_str::<QemuCommands>(&payload) {
            result =  Ok(command);
        }
        //unsubscribe from the channel
        pubsub.unsubscribe(&key).context("Failed to unsubscribe from Redis channel")?;
        return result;
    }

    pub fn send_qemu_event(
        &self,
        competition_name: &str,
        team_name: &str,
        box_name: &str,
        command: QemuCommands,
    ) -> Result<()> {
        let mut conn = self.client.get_connection().context("Failed to connect to Redis")?;
        
        // the key name
        let key = format!("{}:{}:{}:events", competition_name, team_name, box_name);
        
        // Publish the command as a YAML string
        let payload = serde_yaml::to_string(&command).context("Failed to serialize QEMU command")?;
        let _: () = redis::cmd("PUBLISH")
            .arg(&key)
            .arg(payload)
            .query(&mut conn)
            .context("Failed to publish QEMU command")?;
        
        Ok(())
    }

    pub fn record_sucessful_check_result(
        &self,
        competition_name: &str,
        check_name: &str,
        timestamp: DateTime<chrono::Utc>,
        team_id: u64,
        box_name: &str,
        message: &str,
    ) -> Result<String> {
        let mut conn = self.client.get_connection().context("Failed to connect to Redis")?;
        // the key name
        let key = format!("{}:{}:{}", competition_name, team_id, check_name);
        let event = ScoreEvent {
            message: message.to_string(),
            timestamp,
            team_id: team_id, // This can be set to a specific team ID if needed
            score_event_type: check_name.to_string(), // e.g., "icmp_check_1"
            box_name: box_name.to_string(), // Name of the box where the event occurred
        };
        let value = serde_yaml::to_string(&event).context("Failed to serialize score event to YAML")?;
        let timestamp_seconds = timestamp.timestamp();
        let _: () = redis::cmd("ZADD")
            .arg(&key)
            .arg(timestamp_seconds)
            .arg(&value)
            .query(&mut conn)
            .context("Failed to record successful check result")?;
        Ok(key)
    }
    
    
    
    // Get detailed teams scores by check
    pub fn get_team_score_by_check(&self, competition_name: &str, team_id: u64, check_name: &str, check_points : i64) -> Result<i64> {
        let mut conn = self.client.get_connection().context("Failed to connect to Redis")?;
        
        // the key name
        let key = format!("{}:{}:{}", competition_name, team_id, check_name);

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
    pub fn get_team_score_check_events(&self, competition_name: &str, team_id: u64, check_name: &str, time_start : i64, time_end: i64) -> Result<Vec<(ScoreEvent, chrono::DateTime<chrono::Utc>)>> {
        let mut conn = self.client.get_connection().context("Failed to connect to Redis")?;
        // the key name
        let key = format!("{}:{}:{}", competition_name, team_id, check_name);
        // Get the events for this team in this competition
        let events: Vec<String> = redis::cmd("ZRANGEBYSCORE")
            .arg(&key)
            .arg(time_start)
            .arg(time_end)
            .arg("WITHSCORES")
            .query(&mut conn)
            .context("Failed to get team score check events")?;
        let mut result = Vec::new();
        for chunk in events.chunks(2) {
            if chunk.len() == 2 {
                let event: ScoreEvent = match serde_yaml::from_str(&chunk[0]) {
                    Ok(ev) => ev,
                    Err(e) => return Err(anyhow::anyhow!("Failed to deserialize ScoreEvent: {} (raw: {})", e, chunk[0])),
                };
                let timestamp = chunk[1].parse::<i64>().expect("Failed to parse timestamp");
                let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(timestamp, 0).expect("Failed to convert timestamp to DateTime");
                result.push((event, dt));
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
        team_name: Option<&str>
    ) -> Result<()> {
        let mut conn = self.client.get_connection().context("Failed to connect to Redis")?;
        
        // Keys for Redis operations
        let users_key = format!("{}:users", competition_name);
        let user_data = user.to_redis_format();
        
        // Check if user already exists in the competition
        let user_exists: bool = redis::cmd("SISMEMBER")
            .arg(&users_key)
            .arg(&user_data)
            .query(&mut conn)
            .context("Failed to check if user exists")?;
        
        if user_exists {
            // User exists, need to find their current team and move them if a new team is provided
            if let Some(_) = team_name {
                let pattern = format!("{}:*:users", competition_name);
                let team_keys: Vec<String> = redis::cmd("KEYS")
                    .arg(&pattern)
                    .query(&mut conn)
                    .context("Failed to get team keys")?;
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
            }
        } else {
            // New user, add to global users set
            let _: () = redis::cmd("SADD")
                .arg(&users_key)
                .arg(&user_data)
                .query(&mut conn)
                .context("Failed to add user to global users set")?;
        }
        
        // Add user to the new team if provided
        if let Some(team_name) = team_name {
            let new_team_users_key = format!("{}:{}:users", competition_name, team_name);
            let _: () = redis::cmd("SADD")
                .arg(&new_team_users_key)
                .arg(&user_data)
                .query(&mut conn)
                .context("Failed to add user to new team")?;
        }
        
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

    // get the global competition state atomically. If the state is not set, will insert a default state (Unstarted).
    pub fn get_competition_state(&self, competition_name: &str) -> Result<CompetitionState> {
        let mut conn = self.client.get_connection().context("Failed to connect to Redis")?;
        
        // Key for competition state
        let key = format!("{}:state", competition_name);
        
        // Try to get the state
        let state: Option<String> = redis::cmd("HGET")
            .arg(&key)
            .arg("state")
            .query(&mut conn)
            .context("Failed to get competition state")?;
        match state {
            None => {
                let default_state = CompetitionState {
                    name: competition_name.to_string(),
                    status: CompetitionStatus::Unstarted,
                    start_time: None,
                    end_time: None,
                };
                // Insert default state into Redis
                let _: () = redis::cmd("HSET")
                    .arg(&key)
                    .arg("state")
                    .arg(serde_yaml::to_string(&default_state).context("Failed to serialize default state")?)
                    .query(&mut conn)
                    .context("Failed to set default competition state")?;
                Ok(default_state)
            }
            Some(state_str) => {
                match serde_yaml::from_str::<CompetitionState>(&state_str) {
                    Ok(state) => {
                        // If the state is found, return it
                        Ok(state)
                    }
                    Err(e) => Err(anyhow::anyhow!("Failed to deserialize competition state: {}", e)),
                }
            }
        }
    }

    //starts the copmetition. Returns an error if the competition is already started or finished.
    pub fn start_competition(&self, competition_name: &str, duration: Option<u64>) -> Result<()> {
        let mut conn = self.client.get_connection().context("Failed to connect to Redis")?;
        // use get_competition_state to check current state
        let current_state = self.get_competition_state(competition_name)?;
        match current_state.status {
            CompetitionStatus::Active => Err(anyhow::anyhow!("Competition is already active")),
            CompetitionStatus::Unstarted => {
                // Set new state to active with current timestamp
                let start_time = chrono::Utc::now();
                let end_time = duration.map(|d| start_time + chrono::Duration::seconds(d as i64));
                
                let new_state = CompetitionState {
                    name: competition_name.to_string(),
                    status: CompetitionStatus::Active,
                    start_time: Some(start_time),
                    end_time,
                };
                
                // Store the new state in Redis
                let key = format!("{}:state", competition_name);
                let _: () = redis::cmd("HSET")
                    .arg(&key)
                    .arg("state")
                    .arg(serde_yaml::to_string(&new_state).context("Failed to serialize competition state")?)
                    .query(&mut conn)
                    .context("Failed to start competition")?;
                // publish the start event to a channel
                let _: () = redis::cmd("PUBLISH")
                    .arg(format!("{}:events", competition_name))
                    .arg(serde_yaml::to_string(&new_state).context("Failed to serialize competition state for publish")?)
                    .query(&mut conn)
                    .context("Failed to publish competition start event")?;
                Ok(())
            },
            CompetitionStatus::Finished => Err(anyhow::anyhow!("Competition has already finished")),
        }
    }

    // Ends the competition. Returns an error if the competition is not active.
    pub fn end_competition(&self, competition_name: &str) -> Result<()> {
        let mut conn = self.client.get_connection().context("Failed to connect to Redis")?;
        // use get_competition_state to check current state
        let current_state = self.get_competition_state(competition_name)?;
        match current_state.status {
            CompetitionStatus::Active => {
                // Set new state to finished with current timestamp
                let end_time = chrono::Utc::now();
                
                let new_state = CompetitionState {
                    name: competition_name.to_string(),
                    status: CompetitionStatus::Finished,
                    start_time: current_state.start_time,
                    end_time: Some(end_time),
                };
                
                // Store the new state in Redis
                let key = format!("{}:state", competition_name);
                let _: () = redis::cmd("HSET")
                    .arg(&key)
                    .arg("state")
                    .arg(serde_yaml::to_string(&new_state).context("Failed to serialize competition state")?)
                    .query(&mut conn)
                    .context("Failed to end competition")?;
                // publish the end event to a channel
                let _: () = redis::cmd("PUBLISH")
                    .arg(format!("{}:events", competition_name))
                    .arg(serde_yaml::to_string(&new_state).context("Failed to serialize competition state for publish")?)
                    .query(&mut conn)
                    .context("Failed to publish competition end event")?;
                Ok(())
            },
            CompetitionStatus::Unstarted => Err(anyhow::anyhow!("Competition has not started yet")),
            CompetitionStatus::Finished => Err(anyhow::anyhow!("Competition has already finished")),
        }
    }

    pub fn wait_for_competition_event(&self, competition_name: &str) -> Result<CompetitionState> {
        let mut conn = self.client.get_connection().context("Failed to connect to Redis")?;
        
        // Subscribe to the competition events channel
        let mut pubsub = conn.as_pubsub();
        pubsub.subscribe(format!("{}:events", competition_name)).context("Failed to subscribe to competition events")?;
        
        // Wait for a message
        let msg = pubsub.get_message().context("Failed to get message from Redis")?;
        println!("Received message: {:?}", msg);
        let payload: String = msg.get_payload().context("Failed to get payload from message")?;
        println!("Received payload: {}", payload);
        // Deserialize the competition state
        let state: CompetitionState = serde_yaml::from_str(&payload).context("Failed to deserialize competition state")?;
        
        // Unsubscribe from the channel
        pubsub.unsubscribe(format!("{}:events", competition_name)).context("Failed to unsubscribe from competition events")?;
        
        Ok(state)
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
