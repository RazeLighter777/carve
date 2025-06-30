use crate::config::{FlagCheck, RedisConfig};
use crate::util;
use anyhow::{Context, Result};
use argon2::PasswordHasher;
use argon2::{PasswordVerifier, password_hash::SaltString};
use chrono::{DateTime, Utc};
use rand::distr::{Distribution, SampleString};
use redis::Client;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IdentitySources {
    LocalUserPassword,
    OIDC,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct User {
    pub username: String,
    pub email: String,
    pub team_name: Option<String>,
    pub is_admin: bool, // Optional field to indicate if the user is an admin]
    pub identity_sources: Vec<IdentitySources>, // List of identity sources for the user
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
    Restore,
    Stop,
    Snapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScoreEvent {
    pub message: String, // Message describing the event, e.g., "ICMP check passed", "Flag check successful"
    pub timestamp: DateTime<chrono::Utc>,
    pub team_id: u64,
    pub score_event_type: String, // e.g., "icmp_check_1" "flag_check_1"
    pub box_name: String,         // Name of the box where the event occurred
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompetitionState {
    pub name: String,
    pub status: CompetitionStatus,
    pub start_time: Option<DateTime<Utc>>, // Unix timestamp in seconds
    pub end_time: Option<DateTime<Utc>>,   // Unix timestamp in seconds
}

impl User {
    pub fn new(
        username: String,
        email: String,
        identity_sources: impl Iterator<Item = IdentitySources>,
    ) -> Self {
        Self {
            username,
            email,
            team_name: None,
            is_admin: false, // Default to false, can be set later
            identity_sources: identity_sources.collect(), // Collect into a Vec
        }
    }

    pub fn with_team(
        username: String,
        email: String,
        team_name: String,
        identity_sources: impl Iterator<Item = IdentitySources>,
    ) -> Self {
        Self {
            username,
            email,
            team_name: Some(team_name),
            is_admin: false, // Default to false, can be set later
            identity_sources: identity_sources.collect(), // Collect into a Vec
        }
    }

    // Convert user to Redis storage format
    pub fn to_redis_format(&self) -> String {
        //serialize using serde_yaml

        serde_yaml::to_string(self).expect("Failed to serialize user to YAML")
    }

    // Parse user from Redis storage format (username:email)
    pub fn from_redis_format(data: &str) -> Option<Self> {
        // Deserialize using serde_yaml
        serde_yaml::from_str::<Self>(data).ok()
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
        let mut conn = self
            .client
            .get_connection()
            .context("Failed to connect to Redis")?;

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
        let mut conn = self
            .client
            .get_connection()
            .context("Failed to connect to Redis")?;
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

    pub fn check_team_join_code(
        &self,
        competition_name: &str,
        join_code: u64,
    ) -> Result<Option<String>> {
        let mut conn = self
            .client
            .get_connection()
            .context("Failed to connect to Redis")?;

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
        let mut conn = self
            .client
            .get_connection()
            .context("Failed to connect to Redis")?;
        redis::cmd("PING")
            .query::<String>(&mut conn)
            .context("Failed to ping Redis")?;
        Ok(())
    }

    // wait for events for qemu boxes.
    // this blocking call takes an iterator of events, and waits one event to happen.
    pub fn wait_for_qemu_event(
        &self,
        competition_name: &str,
        team_name: &str,
        box_name: &str,
        events: impl Iterator<Item = QemuCommands> + Clone,
    ) -> Result<QemuCommands> {
        let mut conn = self
            .client
            .get_connection()
            .context("Failed to connect to Redis")?;

        // the key name
        let key = format!("{}:{}:{}:events", competition_name, team_name, box_name);

        // Subscribe to the key for events
        let mut pubsub = conn.as_pubsub();
        pubsub
            .subscribe(&key)
            .context("Failed to subscribe to Redis channel")?;

        // Wait for an event
        loop {
            let msg = pubsub
                .get_message()
                .context("Failed to get message from Redis channel")?;
            let payload: String = msg
                .get_payload()
                .context("Failed to get payload from Redis message")?;

            // Deserialize the payload into a QemuCommands
            if let Ok(command) = serde_yaml::from_str::<QemuCommands>(&payload) {
                // Check if the command is in the expected events
                if events.clone().any(|e| e == command) {
                    return Ok(command);
                }
            }
        }
    }

    pub fn send_qemu_event(
        &self,
        competition_name: &str,
        team_name: &str,
        box_name: &str,
        command: QemuCommands,
    ) -> Result<()> {
        let mut conn = self
            .client
            .get_connection()
            .context("Failed to connect to Redis")?;

        // the key name
        let key = format!("{}:{}:{}:events", competition_name, team_name, box_name);

        // Publish the command as a YAML string
        let payload =
            serde_yaml::to_string(&command).context("Failed to serialize QEMU command")?;
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
        let key = format!("{}:{}:{}", competition_name, team_id, check_name);
        // Only record if competition is Active
        let state = self.get_competition_state(competition_name)?;
        if state.status != CompetitionStatus::Active {
            // Do nothing, just return the key name
            return Ok(key);
        }
        let mut conn = self
            .client
            .get_connection()
            .context("Failed to connect to Redis")?;
        let event = ScoreEvent {
            message: message.to_string(),
            timestamp,
            team_id, // This can be set to a specific team ID if needed
            score_event_type: check_name.to_string(), // e.g., "icmp_check_1"
            box_name: box_name.to_string(), // Name of the box where the event occurred
        };
        let value =
            serde_yaml::to_string(&event).context("Failed to serialize score event to YAML")?;
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
    pub fn get_team_score_by_check(
        &self,
        competition_name: &str,
        team_id: u64,
        check_name: &str,
        check_points: i64,
    ) -> Result<i64> {
        let mut conn = self
            .client
            .get_connection()
            .context("Failed to connect to Redis")?;

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
    pub fn get_team_score_check_events(
        &self,
        competition_name: &str,
        team_id: u64,
        check_name: &str,
        time_start: i64,
        time_end: i64,
    ) -> Result<Vec<(ScoreEvent, chrono::DateTime<chrono::Utc>)>> {
        let mut conn = self
            .client
            .get_connection()
            .context("Failed to connect to Redis")?;
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
                    Err(e) => {
                        return Err(anyhow::anyhow!(
                            "Failed to deserialize ScoreEvent: {} (raw: {})",
                            e,
                            chunk[0]
                        ));
                    }
                };
                let timestamp = chunk[1].parse::<i64>().expect("Failed to parse timestamp");
                let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(timestamp, 0)
                    .expect("Failed to convert timestamp to DateTime");
                result.push((event, dt));
            }
        }
        Ok(result)
    }

    // Write SSH keypair for a box. Returns true if written, false if key exists.
    pub fn write_ssh_keypair(
        &self,
        competition_name: &str,
        team_name: &str,
        box_name: &str,
        private_key: &str,
    ) -> Result<bool> {
        let mut conn = self
            .client
            .get_connection()
            .context("Failed to connect to Redis")?;
        let key = format!(
            "{}:{}:{}:ssh_keypair",
            competition_name, team_name, box_name
        );
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
    pub fn read_ssh_keypair(
        &self,
        competition_name: &str,
        team_name: &str,
        box_name: &str,
    ) -> Result<Option<String>> {
        let mut conn = self
            .client
            .get_connection()
            .context("Failed to connect to Redis")?;
        let key = format!(
            "{}:{}:{}:ssh_keypair",
            competition_name, team_name, box_name
        );
        let val: Option<String> = redis::cmd("GET")
            .arg(&key)
            .query(&mut conn)
            .context("Failed to read SSH keypair")?;
        Ok(val)
    }

    // Write username/password for a box. Returns true if written, false if key exists.
    pub fn write_box_credentials(
        &self,
        competition_name: &str,
        team_name: &str,
        box_name: &str,
        username: &str,
        password: &str,
    ) -> Result<bool> {
        let mut conn = self
            .client
            .get_connection()
            .context("Failed to connect to Redis")?;
        let key = format!(
            "{}:{}:{}:credentials",
            competition_name, team_name, box_name
        );
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
    pub fn read_box_credentials(
        &self,
        competition_name: &str,
        team_name: &str,
        box_name: &str,
    ) -> Result<Option<(String, String)>> {
        let mut conn = self
            .client
            .get_connection()
            .context("Failed to connect to Redis")?;
        let key = format!(
            "{}:{}:{}:credentials",
            competition_name, team_name, box_name
        );
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
    pub fn get_all_users(&self, competition_name: &str) -> Result<Vec<User>> {
        let mut conn = self
            .client
            .get_connection()
            .context("Failed to connect to Redis")?;
        let key = format!("{}:users", competition_name);
        let user_data_key = format!("{}:user_data", competition_name);
        // Get all usernames in the competition
        let usernames: Vec<String> = redis::cmd("SMEMBERS")
            .arg(&key)
            .query(&mut conn)
            .context("Failed to get usernames")?;
        let mut users = Vec::new();
        for username in usernames {
            // Get user data for each username
            let user_data_str: String = redis::cmd("HGET")
                .arg(&user_data_key)
                .arg(&username)
                .query(&mut conn)
                .context("Failed to get user data")?;
            if let Some(user) = User::from_redis_format(&user_data_str) {
                users.push(user);
            } else {
                return Err(anyhow::anyhow!(
                    "Failed to deserialize user data for username: {}",
                    username
                ));
            }
        }
        Ok(users)
    }

    // Register a user to a team. Creates/inserts a new key in competition_name:users and competition_name:team_name:users
    // competition_name:users -> set of usernames, emails.
    // takes a User struct and team_name
    // if user already exists, register them to the new team, check iteratively which team they are registered to, and remove them from the old team.
    pub fn register_user(
        &self,
        competition_name: &str,
        user: &User,
        team_name: Option<&str>,
    ) -> Result<()> {
        let mut conn = self
            .client
            .get_connection()
            .context("Failed to connect to Redis")?;
        // Validate user fields
        if util::validate_user_fields(user).is_err() {
            return Err(anyhow::anyhow!("Invalid user fields"));
        }
        // Keys for Redis operations
        let users_key = format!("{}:users", competition_name);
        let users_data_key = format!("{}:user_data", competition_name);

        // Check if user already exists in the competition
        let user_exists: bool = redis::cmd("SISMEMBER")
            .arg(&users_key)
            .arg(&user.username)
            .query(&mut conn)
            .context("Failed to check if user exists")?;

        if user_exists {
            let existing_user_data_str: String = redis::cmd("HGET")
                .arg(&users_data_key)
                .arg(&user.username)
                .query(&mut conn)
                .context("Failed to get existing user data")?;
            let existing_user = User::from_redis_format(&existing_user_data_str)
                .context("Failed to deserialize existing user data")?;
            // User exists, need to find their current team and move them if a new team is provided
            if team_name.is_some() {
                let pattern = format!("{}:*:users", competition_name);
                let team_keys: Vec<String> = redis::cmd("KEYS")
                    .arg(&pattern)
                    .query(&mut conn)
                    .context("Failed to get team keys")?;
                for team_key in team_keys {
                    let is_member: bool = redis::cmd("SISMEMBER")
                        .arg(&team_key)
                        .arg(&user.username)
                        .query(&mut conn)
                        .context("Failed to check team membership")?;
                    if is_member {
                        // Remove user from old team
                        let _: () = redis::cmd("SREM")
                            .arg(&team_key)
                            .arg(&user.username)
                            .query(&mut conn)
                            .context("Failed to remove user from old team")?;
                        break;
                    }
                }
            }
            // Update <competition_name>:user_data field
            // Make sure new identity_sources are included, make sure to not overwrite existing ones or add duplicates

            // Add new identity source if not already present
            let mut updated_user = existing_user.clone();
            for new_identity_source in user.identity_sources.clone() {
                if !updated_user.identity_sources.contains(&new_identity_source) {
                    updated_user.identity_sources.push(new_identity_source);
                }
            }
            // update email and team name
            updated_user.email = user.email.clone();
            updated_user.team_name = user.team_name.clone();
            // write the updated user data back to Redis
            let user_data = updated_user.to_redis_format();
            let _: () = redis::cmd("HSET")
                .arg(&users_data_key)
                .arg(&user.username)
                .arg(&user_data)
                .query(&mut conn)
                .context("Failed to update user data")?;
        } else {
            // New user, add to global users set
            let _: () = redis::cmd("SADD")
                .arg(&users_key)
                .arg(&user.username)
                .query(&mut conn)
                .context("Failed to add user to global users set")?;
            // Serialize user data
            let user_data = user.to_redis_format();
            // Store user data in <competition_name>:user_data hash
            let _: () = redis::cmd("HSET")
                .arg(&users_data_key)
                .arg(&user.username)
                .arg(&user_data)
                .query(&mut conn)
                .context("Failed to store user data")?;
        }

        // Add user to the new team if provided
        if let Some(team_name) = team_name {
            let new_team_users_key = format!("{}:{}:users", competition_name, team_name);
            let _: () = redis::cmd("SADD")
                .arg(&new_team_users_key)
                .arg(&user.username)
                .query(&mut conn)
                .context("Failed to add user to new team")?;
        }

        Ok(())
    }

    // Set a local user password for a user. This is used for local authentication.
    // the password is hashed with argon2i and stored in Redis competition_name:users:password_hashes where the key is the username and the value is the hashed password.
    // The user will be re-registered with the new identity source LocalUserPassword, copying all other existing identity sources and user data.
    // the method will fail if the user does not exist.
    pub fn set_user_local_password(
        &self,
        competition_name: &str,
        username: &str,
        password: &str,
    ) -> Result<()> {
        let mut conn = self
            .client
            .get_connection()
            .context("Failed to connect to Redis")?;
        // Validate username and password
        if util::validate_password(password).is_err() {
            return Err(anyhow::anyhow!("Invalid password format"));
        }
        // Key for storing user password hashes
        let password_hashes_key = format!("{}:users:password_hashes", competition_name);
        let user_key = format!("{}:users", competition_name);
        // Check if the user exists
        let user_exists: bool = redis::cmd("SISMEMBER")
            .arg(&user_key)
            .arg(username)
            .query(&mut conn)
            .context("Failed to check if user exists")?;

        if !user_exists {
            return Err(anyhow::anyhow!("User does not exist"));
        }
        let hasher = argon2::Argon2::default();
        // Hash the password using argon2i
        let mut rng = argon2::password_hash::rand_core::OsRng;
        let hashed_password = hasher
            .hash_password(password.as_bytes(), &SaltString::generate(&mut rng))
            .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?
            .to_string();

        // Store the hashed password in Redis
        let _: () = redis::cmd("HSET")
            .arg(&password_hashes_key)
            .arg(username)
            .arg(hashed_password)
            .query(&mut conn)
            .context("Failed to store user password hash")?;

        // Re-register the user with the new identity source LocalUserPassword
        let user_data_key = format!("{}:user_data", competition_name);
        let user_data_str: String = redis::cmd("HGET")
            .arg(&user_data_key)
            .arg(username)
            .query(&mut conn)
            .context("Failed to get user data")?;

        if let Some(mut user) = User::from_redis_format(&user_data_str) {
            // Add LocalUserPassword identity source if not already present
            if !user
                .identity_sources
                .contains(&IdentitySources::LocalUserPassword)
            {
                user.identity_sources
                    .push(IdentitySources::LocalUserPassword);
                // Update the user data in Redis
                let updated_user_data = user.to_redis_format();
                let _: () = redis::cmd("HSET")
                    .arg(&user_data_key)
                    .arg(username)
                    .arg(updated_user_data)
                    .query(&mut conn)
                    .context("Failed to update user data with new identity source")?;
            }
        }

        Ok(())
    }

    // verify a local user password for a user. This is used for local authentication.
    // the method will return the User's object if the username/password combination is valid, false otherwise.
    pub fn verify_user_local_password(
        &self,
        competition_name: &str,
        username: &str,
        password: &str,
    ) -> Result<Option<User>> {
        let mut conn = self
            .client
            .get_connection()
            .context("Failed to connect to Redis")?;

        // Key for storing user password hashes
        let password_hashes_key = format!("{}:users:password_hashes", competition_name);

        // Get the hashed password for the user
        let hashed_password: Option<String> = redis::cmd("HGET")
            .arg(&password_hashes_key)
            .arg(username)
            .query(&mut conn)
            .context("Failed to get user password hash")?;

        if let Some(hashed_password) = hashed_password {
            // Verify the password using argon2i
            let hashed_password = argon2::password_hash::PasswordHash::new(&hashed_password)
                .map_err(|e| anyhow::anyhow!("Failed to parse hashed password: {}", e))?;
            let hasher = argon2::Argon2::default();
            if hasher
                .verify_password(password.as_bytes(), &hashed_password)
                .is_ok()
            {
                // Password is valid, get the user data
                let user_data_key = format!("{}:user_data", competition_name);
                let user_data_str: String = redis::cmd("HGET")
                    .arg(&user_data_key)
                    .arg(username)
                    .query(&mut conn)
                    .context("Failed to get user data")?;

                if let Some(user) = User::from_redis_format(&user_data_str) {
                    return Ok(Some(user));
                }
            }
        }

        Ok(None) // Invalid username/password combination
    }

    // Get all users for a team
    pub fn get_team_users(&self, competition_name: &str, team_name: &str) -> Result<Vec<User>> {
        let mut conn = self
            .client
            .get_connection()
            .context("Failed to connect to Redis")?;

        let team_users_key = format!("{}:{}:users", competition_name, team_name);
        let users: Vec<String> = redis::cmd("SMEMBERS")
            .arg(&team_users_key)
            .query(&mut conn)
            .context("Failed to get team users")?;

        let result = users
            .into_iter()
            .filter_map(|user_name| {
                // use redis.get_user_data to get the user data
                if let Ok(Some(userdata)) = self.get_user(competition_name, &user_name) {
                    Some(userdata)
                } else {
                    None
                }
            })
            .collect();

        Ok(result)
    }

    // get the global competition state atomically. If the state is not set, will insert a default state (Unstarted).
    pub fn get_competition_state(&self, competition_name: &str) -> Result<CompetitionState> {
        let mut conn = self
            .client
            .get_connection()
            .context("Failed to connect to Redis")?;

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
                    .arg(
                        serde_yaml::to_string(&default_state)
                            .context("Failed to serialize default state")?,
                    )
                    .query(&mut conn)
                    .context("Failed to set default competition state")?;
                Ok(default_state)
            }
            Some(state_str) => {
                match serde_yaml::from_str::<CompetitionState>(&state_str) {
                    Ok(mut state) => {
                        // If the state is Active and end_time is set and now >= end_time, set to Finished
                        if state.status == CompetitionStatus::Active {
                            if let Some(end_time) = state.end_time {
                                let now = chrono::Utc::now();
                                if now >= end_time {
                                    state.status = CompetitionStatus::Finished;
                                    state.end_time = Some(end_time); // keep the original end_time
                                    // Store the updated state in Redis
                                    let _: () = redis::cmd("HSET")
                                        .arg(&key)
                                        .arg("state")
                                        .arg(
                                            serde_yaml::to_string(&state)
                                                .context("Failed to serialize finished state")?,
                                        )
                                        .query(&mut conn)
                                        .context(
                                            "Failed to update competition state to finished",
                                        )?;
                                    // Optionally publish the finished event
                                    let _: () = redis::cmd("PUBLISH")
                                        .arg(format!("{}:events", competition_name))
                                        .arg(serde_yaml::to_string(&state).context(
                                            "Failed to serialize finished state for publish",
                                        )?)
                                        .query(&mut conn)
                                        .context("Failed to publish competition finished event")?;
                                }
                            }
                        }
                        Ok(state)
                    }
                    Err(e) => Err(anyhow::anyhow!(
                        "Failed to deserialize competition state: {}",
                        e
                    )),
                }
            }
        }
    }

    //starts the copmetition. Returns an error if the competition is already started or finished.
    pub fn start_competition(&self, competition_name: &str, duration: Option<u64>) -> Result<()> {
        let mut conn = self
            .client
            .get_connection()
            .context("Failed to connect to Redis")?;
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
                    .arg(
                        serde_yaml::to_string(&new_state)
                            .context("Failed to serialize competition state")?,
                    )
                    .query(&mut conn)
                    .context("Failed to start competition")?;
                // publish the start event to a channel
                let _: () = redis::cmd("PUBLISH")
                    .arg(format!("{}:events", competition_name))
                    .arg(
                        serde_yaml::to_string(&new_state)
                            .context("Failed to serialize competition state for publish")?,
                    )
                    .query(&mut conn)
                    .context("Failed to publish competition start event")?;
                Ok(())
            }
            CompetitionStatus::Finished => Err(anyhow::anyhow!("Competition has already finished")),
        }
    }

    // Ends the competition. Returns an error if the competition is not active.
    pub fn end_competition(&self, competition_name: &str) -> Result<()> {
        let mut conn = self
            .client
            .get_connection()
            .context("Failed to connect to Redis")?;
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
                    .arg(
                        serde_yaml::to_string(&new_state)
                            .context("Failed to serialize competition state")?,
                    )
                    .query(&mut conn)
                    .context("Failed to end competition")?;
                // publish the end event to a channel
                let _: () = redis::cmd("PUBLISH")
                    .arg(format!("{}:events", competition_name))
                    .arg(
                        serde_yaml::to_string(&new_state)
                            .context("Failed to serialize competition state for publish")?,
                    )
                    .query(&mut conn)
                    .context("Failed to publish competition end event")?;
                Ok(())
            }
            CompetitionStatus::Unstarted => Err(anyhow::anyhow!("Competition has not started yet")),
            CompetitionStatus::Finished => Err(anyhow::anyhow!("Competition has already finished")),
        }
    }

    pub fn wait_for_competition_event(&self, competition_name: &str) -> Result<CompetitionState> {
        let mut conn = self
            .client
            .get_connection()
            .context("Failed to connect to Redis")?;

        // Subscribe to the competition events channel
        let mut pubsub = conn.as_pubsub();
        pubsub
            .subscribe(format!("{}:events", competition_name))
            .context("Failed to subscribe to competition events")?;

        // Wait for a message
        let msg = pubsub
            .get_message()
            .context("Failed to get message from Redis")?;
        println!("Received message: {:?}", msg);
        let payload: String = msg
            .get_payload()
            .context("Failed to get payload from message")?;
        println!("Received payload: {}", payload);
        // Deserialize the competition state
        let state: CompetitionState =
            serde_yaml::from_str(&payload).context("Failed to deserialize competition state")?;

        // Unsubscribe from the channel
        pubsub
            .unsubscribe(format!("{}:events", competition_name))
            .context("Failed to unsubscribe from competition events")?;

        Ok(state)
    }

    pub fn generate_new_flag(
        &self,
        competition_name: &str,
        team_name: &str,
        flag_check_name: &str,
    ) -> Result<String> {
        let mut conn = self
            .client
            .get_connection()
            .context("Failed to connect to Redis")?;

        // Key for storing flags
        let key = format!(
            "{}:{}:{}:flags",
            competition_name, team_name, flag_check_name
        );
        // Generate a new flag in the format "<competition_name>{random 8 lowercase alphabetic characters}"
        let value: String = format!(
            "{}{{{}}}",
            competition_name,
            rand::distr::Alphabetic
                .sample_string(&mut rand::rng(), 8)
                .to_lowercase()
        );
        // Store the flag in Redis
        let _: () = redis::cmd("SADD")
            .arg(&key)
            .arg(&value)
            .query(&mut conn)
            .context("Failed to register new flag")?;
        Ok(value)
    }

    pub fn redeem_flag(
        &self,
        competition_name: &str,
        team_name: &str,
        team_id: u64,
        flag: &str,
        flag_check: &FlagCheck,
    ) -> Result<bool> {
        let mut conn = self
            .client
            .get_connection()
            .context("Failed to connect to Redis")?;

        // Key for storing flags
        let key = format!(
            "{}:{}:{}:flags",
            competition_name, team_name, flag_check.name
        );
        // Check if the flag exists
        let exists: bool = redis::cmd("SISMEMBER")
            .arg(&key)
            .arg(flag)
            .query(&mut conn)
            .context("Failed to check if flag exists")?;

        // create score event for the flag redemption
        if exists {
            // Record the successful flag redemption
            let timestamp = chrono::Utc::now();
            let event_message = format!("Flag redeemed: {}", flag);
            self.record_sucessful_check_result(
                competition_name,
                &flag_check.name,
                timestamp,
                team_id,
                &flag_check.box_name,
                &event_message,
            )?;
            // set the current state of the flag check to true
            self.set_check_current_state(
                competition_name,
                team_name,
                &flag_check.name,
                true,
                0, // No failures on successful flag redemption
                &event_message,
            )?;
        }

        if exists {
            // Remove the flag from the set
            let _: () = redis::cmd("SREM")
                .arg(&key)
                .arg(flag)
                .query(&mut conn)
                .context("Failed to remove flag from set")?;
            Ok(true) // Flag redeemed successfully
        } else {
            Ok(false) // Flag does not exist
        }
    }

    pub fn set_check_current_state(
        &self,
        competition_name: &str,
        team_name: &str,
        check_name_or_flag_check_name: &str,
        success: bool,
        number_of_failures: u64,
        message: &str,
    ) -> Result<()> {
        let mut conn = self
            .client
            .get_connection()
            .context("Failed to connect to Redis")?;

        // Key for storing the current state of the flag check
        let key = format!("{}:{}:current_state", competition_name, team_name);
        let key2 = check_name_or_flag_check_name.to_string();
        let status = format!("{}:{}:{}", success as u64, number_of_failures, message);
        // Store the current state as a u64 (0 for false, 1 for true)
        let _: () = redis::cmd("HSET")
            .arg(&key)
            .arg(key2)
            .arg(status)
            .query(&mut conn)
            .context("Failed to set current state")?;

        Ok(())
    }

    pub fn get_check_current_state(
        &self,
        competition_name: &str,
        team_name: &str,
        check_name_or_flag_check_name: &str,
    ) -> Result<Option<(bool, u64, String)>> {
        let mut conn = self
            .client
            .get_connection()
            .context("Failed to connect to Redis")?;
        // Key for storing the current state of the flag check
        let key = format!("{}:{}:current_state", competition_name, team_name);
        // Get the current state for the specified check
        let state: Option<String> = redis::cmd("HGET")
            .arg(&key)
            .arg(check_name_or_flag_check_name)
            .query(&mut conn)
            .context("Failed to get current state")?;
        if let Some(state_str) = state {
            let parts: Vec<&str> = state_str.split(':').collect();
            if parts.len() >= 3 {
                if let (Ok(success), Ok(failures)) =
                    (parts[0].parse::<u64>(), parts[1].parse::<u64>())
                {
                    return Ok(Some((success != 0, failures, parts[2..].join(":"))));
                }
                return Err(anyhow::anyhow!("Invalid state format: {}", state_str));
            }
            return Err(anyhow::anyhow!("Invalid state format: {}", state_str));
        }
        Ok(Some((false, 0, String::from("Unsolved")))) // No state found
    }

    // Get a specific user by username and find their team
    pub fn get_user(&self, competition_name: &str, username: &str) -> Result<Option<User>> {
        let mut conn = self
            .client
            .get_connection()
            .context("Failed to connect to Redis")?;

        // Get all users to find the one with matching username
        let users_key = format!("{}:users", competition_name);
        let user_data_key = format!("{}:user_data", competition_name);
        if redis::cmd("SISMEMBER")
            .arg(&users_key)
            .arg(username)
            .query(&mut conn)
            .context("Failed to check if user exists")?
        {
            // User exists, get their data
            let user_data: Option<String> = redis::cmd("HGET")
                .arg(&user_data_key)
                .arg(username)
                .query(&mut conn)
                .context("Failed to get user data")?;

            return if let Some(data) = user_data {
                Ok(User::from_redis_format(&data))
            } else {
                Ok(None) // User data not found
            };
        }
        Ok(None)
    }
}
