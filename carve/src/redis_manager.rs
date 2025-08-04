use rand::{Rng, rng};
use std::net::IpAddr;

use crate::config::{FlagCheck, RedisConfig};
use crate::util;
use anyhow::{Context, Result};
use argon2::PasswordVerifier;
use chrono::{DateTime, Utc};
use futures_util::StreamExt;
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
pub struct CompetitionState {
    pub name: String,
    pub status: CompetitionStatus,
    pub start_time: Option<DateTime<Utc>>, // Unix timestamp in seconds
    pub end_time: Option<DateTime<Utc>>,   // Unix timestamp in seconds
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CheckCurrentState {
    pub success: bool,
    pub number_of_failures: u64,
    pub message: Vec<String>,
    pub success_fraction: (u64, u64), // fraction of successful checks over total checks
    pub passing_boxes: Vec<String>,   // List of boxes that passed the check
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
    // Helper to get Redis connection
    async fn get_connection(&self) -> Result<redis::aio::MultiplexedConnection> {
        self.client
            .get_multiplexed_tokio_connection()
            .await
            .context("Failed to connect to Redis")
    }

    // Key helpers
    fn competition_key(&self, competition_name: &str, suffix: &str) -> String {
        format!("{}:{}", competition_name, suffix)
    }
    fn team_key(&self, competition_name: &str, team_name: &str, suffix: &str) -> String {
        format!("{}:{}:{}", competition_name, team_name, suffix)
    }
    fn box_key(
        &self,
        competition_name: &str,
        team_name: &str,
        box_name: &str,
        suffix: &str,
    ) -> String {
        format!("{}:{}:{}:{}", competition_name, team_name, box_name, suffix)
    }

    // Redis command helpers
    async fn redis_hset<K: redis::ToRedisArgs, F: redis::ToRedisArgs, V: redis::ToRedisArgs>(
        &self,
        key: K,
        field: F,
        value: V,
    ) -> Result<()> {
        let mut conn = self.get_connection().await?;
        redis::cmd("HSET")
            .arg(key)
            .arg(field)
            .arg(value)
            .query_async(&mut conn)
            .await
            .context("Failed to execute HSET")
    }
    async fn redis_hget<K: redis::ToRedisArgs, F: redis::ToRedisArgs, T: redis::FromRedisValue>(
        &self,
        key: K,
        field: F,
    ) -> Result<Option<T>> {
        let mut conn = self.get_connection().await?;
        redis::cmd("HGET")
            .arg(key)
            .arg(field)
            .query_async(&mut conn)
            .await
            .context("Failed to execute HGET")
    }
    async fn redis_sadd<K: redis::ToRedisArgs, V: redis::ToRedisArgs>(
        &self,
        key: K,
        value: V,
    ) -> Result<()> {
        let mut conn = self.get_connection().await?;
        redis::cmd("SADD")
            .arg(key)
            .arg(value)
            .query_async(&mut conn)
            .await
            .context("Failed to execute SADD")
    }

    // Random generation helpers
    fn generate_hex_string(length: usize) -> String {
        let mut rng = rng();
        (0..length)
            .map(|_| format!("{:02x}", rng.random::<u8>()))
            .collect()
    }

    fn generate_alphanumeric_string(length: usize) -> String {
        let chars = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
        let mut rng = rng();
        (0..length)
            .map(|_| chars[rng.random_range(0..chars.len())] as char)
            .collect()
    }

    fn generate_lowercase_string(length: usize) -> String {
        let chars = b"abcdefghijklmnopqrstuvwxyz";
        let mut rng = rng();
        (0..length)
            .map(|_| chars[rng.random_range(0..chars.len())] as char)
            .collect()
    }

    // Serialization helpers
    fn serialize_to_yaml<T: serde::Serialize>(value: &T) -> Result<String> {
        serde_yaml::to_string(value).context("Failed to serialize to YAML")
    }
    fn deserialize_from_yaml<T: for<'de> serde::Deserialize<'de>>(yaml: &str) -> Result<T> {
        serde_yaml::from_str(yaml).context("Failed to deserialize from YAML")
    }
    pub fn new(config: &RedisConfig) -> Result<Self> {
        let redis_url = format!("redis://{}:{}/{}", config.host, config.port, config.db);
        let client = Client::open(redis_url).context("Failed to create Redis client")?;
        Ok(Self { client })
    }

    pub async fn generate_team_join_code(
        &self,
        competition_name: &str,
        team_name: &str,
    ) -> Result<u64> {
        let mut conn = self.get_connection().await?;
        let join_code: u64 = rand::random::<u64>() % 1_000_000_000;
        let key = self.competition_key(competition_name, "team_join_codes");

        self.redis_hset(&key, join_code, team_name).await?;

        redis::cmd("HEXPIRE")
            .arg(&key)
            .arg(86400)
            .arg("FIELDS")
            .arg(1)
            .arg(join_code)
            .query_async::<()>(&mut conn)
            .await
            .context("Failed to set expiration for team join code")?;
        Ok(join_code)
    }
    // Generates a box console code for a team. This is a unique code that can be used to access the team's boxes,
    // and is passed to novnc proxy in the url path.
    // This is a 32 character alphanumeric code.
    // if the code already exists, it will return the existing code.
    pub async fn get_box_console_code(
        &self,
        competition_name: &str,
        team_name: &str,
    ) -> Result<String> {
        let key = self.competition_key(competition_name, "box_console_codes");

        if let Some(console_code) = self.redis_hget::<_, _, String>(&key, team_name).await? {
            return Ok(console_code);
        }

        let console_code = Self::generate_alphanumeric_string(32);
        self.redis_hset(&key, team_name, &console_code).await?;
        Ok(console_code)
    }

    pub async fn check_team_join_code(
        &self,
        competition_name: &str,
        join_code: u64,
    ) -> Result<Option<String>> {
        let key = self.competition_key(competition_name, "team_join_codes");
        self.redis_hget(&key, join_code).await
    }

    pub async fn health_check(&self) -> Result<()> {
        let mut conn = self.get_connection().await?;
        redis::cmd("PING")
            .query_async::<String>(&mut conn)
            .await
            .context("Failed to ping Redis")?;
        Ok(())
    }

    // wait for events for qemu boxes.
    // this blocking call takes an iterator of events, and waits one event to happen.
    pub async fn wait_for_qemu_event(
        &self,
        competition_name: &str,
        team_name: &str,
        box_name: &str,
        events: impl Iterator<Item = QemuCommands> + Clone,
    ) -> Result<QemuCommands> {
        // the key name
        let key = format!("{}:{}:{}:events", competition_name, team_name, box_name);

        // Subscribe to the key for events
        let (mut sink, mut stream) = self
            .client
            .get_async_pubsub()
            .await
            .context("Failed to get Redis pubsub connection")?
            .split();
        sink.subscribe(&key)
            .await
            .context("Failed to subscribe to Redis channel")?;

        // Return next event that matches one of the commands
        loop {
            let msg = stream
                .next()
                .await
                .context("Failed to receive message from Redis")?;
            // check if the message is a valid QEMU command
            if let Ok(command) = serde_yaml::from_str::<QemuCommands>(
                &msg.get_payload::<String>()
                    .context("Failed to get payload from Redis message")?,
            ) {
                if events.clone().any(|e| e == command) {
                    return Ok(command);
                }
            }
        }
    }

    pub async fn send_qemu_event(
        &self,
        competition_name: &str,
        team_name: &str,
        box_name: &str,
        command: QemuCommands,
    ) -> Result<()> {
        let mut conn = self
            .client
            .get_multiplexed_tokio_connection()
            .await
            .context("Failed to connect to Redis")?;

        // the key name
        let key = format!("{}:{}:{}:events", competition_name, team_name, box_name);

        // Publish the command as a YAML string
        let payload =
            serde_yaml::to_string(&command).context("Failed to serialize QEMU command")?;
        let _: () = redis::cmd("PUBLISH")
            .arg(&key)
            .arg(payload)
            .query_async(&mut conn)
            .await
            .context("Failed to publish QEMU command")?;

        Ok(())
    }

    pub async fn create_cooldown(
        &self,
        competition_name: &str,
        team_name: &str,
        box_name: &str,
        cooldown_seconds: u64,
    ) -> Result<()> {
        let mut conn = self
            .client
            .get_multiplexed_tokio_connection()
            .await
            .context("Failed to connect to Redis")?;

        // the key name
        let key = format!("{}:{}:{}:cooldown", competition_name, team_name, box_name);

        // Set the cooldown with an expiration time
        let _: () = redis::cmd("SET")
            .arg(&key)
            .arg("active")
            .arg("EX")
            .arg(cooldown_seconds)
            .query_async(&mut conn)
            .await
            .context("Failed to create cooldown")?;

        Ok(())
    }

    pub async fn is_cooldown_ready(
        &self,
        competition_name: &str,
        team_name: &str,
        box_name: &str,
    ) -> Option<i64> {
        // check if key is expiring, and if it is return time left with TTL
        let mut conn = match self.client.get_multiplexed_tokio_connection().await {
            Ok(conn) => conn,
            Err(_) => return None, // Return None if connection fails
        };
        // the key name
        let key = format!("{}:{}:{}:cooldown", competition_name, team_name, box_name);
        // Check if the cooldown key exists
        let ttl: i64 = redis::cmd("TTL")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .context("Failed to check cooldown TTL")
            .ok()?;
        // redis returns -2 if the key does not exist, -1 if it exists but has no expiration
        if ttl == -2 {
            return None; // Cooldown does not exist
        } else if ttl == -1 {
            return Some(0); // Cooldown exists but has no expiration
        }
        // If the key exists, return the remaining TTL
        Some(ttl) // Return the remaining TTL in seconds
    }

    pub async fn create_vxlan_fdb_entry(
        &self,
        competition_name: &str,
        mac_address: &str,
        ip_address: IpAddr,
        domain: &str,
    ) -> Result<()> {
        let mut conn = self.get_connection().await?;
        let key = format!("{}:vxlan_fdb:{}", competition_name, domain);

        self.redis_hset(&key, mac_address, ip_address.to_string())
            .await?;

        redis::cmd("HEXPIRE")
            .arg(&key)
            .arg(20)
            .arg("FIELDS")
            .arg(1)
            .arg(mac_address)
            .query_async::<()>(&mut conn)
            .await
            .context("Failed to set expiration for VXLAN FDB entry")?;
        Ok(())
    }

    pub async fn get_domain_fdb_entries(
        &self,
        competition_name: &str,
        domain: &str,
    ) -> Result<Vec<(String, String)>> {
        let mut conn = self.get_connection().await?;
        let key = format!("{}:vxlan_fdb:{}", competition_name, domain);

        let entries: Vec<String> = redis::cmd("HGETALL")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .context("Failed to get VXLAN FDB entries")?;

        Ok(entries
            .chunks(2)
            .map(|chunk| (chunk[0].to_string(), chunk[1].to_string()))
            .collect())
    }

    pub async fn record_sucessful_check_result(
        &self,
        competition_name: &str,
        check_name: &str,
        timestamp: DateTime<chrono::Utc>,
        team_id: u64,
        occurances: u64,
    ) -> Result<String> {
        let key = format!("{}:{}:{}", competition_name, team_id, check_name);
        // Only record if competition is Active
        let state = self.get_competition_state(competition_name).await?;
        if state.status != CompetitionStatus::Active {
            // Do nothing, just return the key name
            return Ok(key);
        }
        let mut conn = self
            .client
            .get_multiplexed_tokio_connection()
            .await
            .context("Failed to connect to Redis")?;
        let timestamp_seconds = timestamp.timestamp();
        for i in 0..occurances {
            let _: () = redis::cmd("ZADD")
                .arg(&key)
                .arg(timestamp_seconds)
                .arg(format!("{}:{}", timestamp_seconds, i))
                .query_async(&mut conn)
                .await
                .context("Failed to record successful check result")?;
        }
        Ok(key)
    }

    // Get detailed teams scores by check
    pub async fn get_team_score_by_check(
        &self,
        competition_name: &str,
        team_id: u64,
        check_name: &str,
        check_points: i64,
    ) -> Result<i64> {
        let mut conn = self
            .client
            .get_multiplexed_tokio_connection()
            .await
            .context("Failed to connect to Redis")?;

        // the key name
        let key = format!("{}:{}:{}", competition_name, team_id, check_name);

        // Get the total score for this team in this competition
        let score: i64 = redis::cmd("ZCARD")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .context("Failed to get team score")?;

        // multiply by the number of points per check
        let score = score * check_points;

        Ok(score)
    }

    pub async fn record_box_ip(
        &self,
        competition_name: &str,
        team_name: &str,
        box_name: &str,
        ip_address: IpAddr,
    ) -> Result<()> {
        let key = self.box_key(competition_name, team_name, box_name, "ip_address");
        let mut conn = self.get_connection().await?;
        redis::cmd("SET")
            .arg(&key)
            .arg(ip_address.to_string())
            .query_async::<()>(&mut conn)
            .await
            .context("Failed to record box IP address")
    }

    // Helper method for box data operations
    async fn write_box_data(
        &self,
        competition_name: &str,
        team_name: &str,
        box_name: &str,
        suffix: &str,
        data: &str,
    ) -> Result<bool> {
        let mut conn = self.get_connection().await?;
        let key = self.box_key(competition_name, team_name, box_name, suffix);
        let res: Option<String> = redis::cmd("SET")
            .arg(&key)
            .arg(data)
            .arg("NX")
            .query_async(&mut conn)
            .await
            .with_context(|| format!("Failed to write box {}", suffix))?;
        Ok(res.is_some())
    }

    async fn read_box_data(
        &self,
        competition_name: &str,
        team_name: &str,
        box_name: &str,
        suffix: &str,
    ) -> Result<Option<String>> {
        let mut conn = self.get_connection().await?;
        let key = self.box_key(competition_name, team_name, box_name, suffix);
        redis::cmd("GET")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .with_context(|| format!("Failed to read box {}", suffix))
    }

    // Write SSH keypair for a box. Returns true if written, false if key exists.
    pub async fn write_ssh_keypair(
        &self,
        competition_name: &str,
        team_name: &str,
        box_name: &str,
        private_key: &str,
    ) -> Result<bool> {
        self.write_box_data(
            competition_name,
            team_name,
            box_name,
            "ssh_keypair",
            private_key,
        )
        .await
    }

    // Read SSH keypair for a box. Returns None if not found.
    pub async fn read_ssh_keypair(
        &self,
        competition_name: &str,
        team_name: &str,
        box_name: &str,
    ) -> Result<Option<String>> {
        self.read_box_data(competition_name, team_name, box_name, "ssh_keypair")
            .await
    }

    // Write username/password for a box. Returns true if written, false if key exists.
    pub async fn write_box_credentials(
        &self,
        competition_name: &str,
        team_name: &str,
        box_name: &str,
        username: &str,
        password: &str,
    ) -> Result<bool> {
        let value = format!("{}:{}", username, password);
        self.write_box_data(competition_name, team_name, box_name, "credentials", &value)
            .await
    }

    // Read username/password for a box. Returns None if not found.
    pub async fn read_box_credentials(
        &self,
        competition_name: &str,
        team_name: &str,
        box_name: &str,
    ) -> Result<Option<(String, String)>> {
        if let Some(val) = self
            .read_box_data(competition_name, team_name, box_name, "credentials")
            .await?
        {
            let mut parts = val.splitn(2, ':');
            if let (Some(username), Some(password)) = (parts.next(), parts.next()) {
                return Ok(Some((username.to_string(), password.to_string())));
            }
        }
        Ok(None)
    }
    pub async fn get_all_users(&self, competition_name: &str) -> Result<Vec<User>> {
        let mut conn = self
            .client
            .get_multiplexed_tokio_connection()
            .await
            .context("Failed to connect to Redis")?;
        let key = format!("{}:users", competition_name);
        let user_data_key = format!("{}:user_data", competition_name);
        // Get all usernames in the competition
        let usernames: Vec<String> = redis::cmd("SMEMBERS")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .context("Failed to get usernames")?;
        let mut users = Vec::new();
        for username in usernames {
            // Get user data for each username
            let user_data_str: String = redis::cmd("HGET")
                .arg(&user_data_key)
                .arg(&username)
                .query_async(&mut conn)
                .await
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
    pub async fn register_user(
        &self,
        competition_name: &str,
        user: &User,
        team_name: Option<&str>,
    ) -> Result<()> {
        util::validate_user_fields(user)
            .map_err(|e| anyhow::anyhow!("Invalid user fields: {}", e))?;
        let users_key = self.competition_key(competition_name, "users");
        let users_data_key = self.competition_key(competition_name, "user_data");

        let mut updated_user = if let Some(existing_user_data) = self
            .redis_hget::<_, _, String>(&users_data_key, &user.username)
            .await?
        {
            let mut existing_user = User::from_redis_format(&existing_user_data)
                .context("Failed to deserialize existing user data")?;
            for new_source in &user.identity_sources {
                if !existing_user.identity_sources.contains(new_source) {
                    existing_user.identity_sources.push(new_source.clone());
                }
            }
            existing_user.email = user.email.clone();
            existing_user.team_name = user.team_name.clone();
            existing_user
        } else {
            self.redis_sadd(&users_key, &user.username).await?;
            user.clone()
        };

        if let Some(team_name) = team_name {
            self.move_user_to_team(competition_name, &user.username, team_name)
                .await?;
            updated_user.team_name = Some(team_name.to_string());
        }

        self.redis_hset(
            &users_data_key,
            &user.username,
            updated_user.to_redis_format(),
        )
        .await?;
        Ok(())
    }

    async fn move_user_to_team(
        &self,
        competition_name: &str,
        username: &str,
        new_team: &str,
    ) -> Result<()> {
        let mut conn = self.get_connection().await?;
        let pattern = format!("{}:*:users", competition_name);
        let team_keys: Vec<String> = redis::cmd("KEYS")
            .arg(&pattern)
            .query_async(&mut conn)
            .await?;
        for team_key in team_keys {
            let _: () = redis::cmd("SREM")
                .arg(&team_key)
                .arg(username)
                .query_async(&mut conn)
                .await?;
        }
        let new_team_key = self.team_key(competition_name, new_team, "users");
        self.redis_sadd(&new_team_key, username).await?;
        Ok(())
    }

    // Set a local user password using argon2i hashing
    pub async fn set_user_local_password(
        &self,
        competition_name: &str,
        username: &str,
        password: &str,
    ) -> Result<()> {
        use argon2::{Argon2, PasswordHasher};
        use argon2::password_hash::{rand_core::OsRng, SaltString};
        
        let password_hashes_key = self.competition_key(competition_name, "users:password_hashes");
        
        // Generate a salt and hash the password
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?
            .to_string();
            
        self.redis_hset(&password_hashes_key, username, password_hash).await?;
        
        // Re-register the user with the new identity source LocalUserPassword
        let user_data_key = self.competition_key(competition_name, "user_data");
        if let Some(user_data_str) = self.redis_hget::<_, _, String>(&user_data_key, username).await? {
            if let Some(mut user) = User::from_redis_format(&user_data_str) {
                if !user.identity_sources.contains(&IdentitySources::LocalUserPassword) {
                    user.identity_sources.push(IdentitySources::LocalUserPassword);
                    self.redis_hset(&user_data_key, username, user.to_redis_format()).await?;
                }
            }
        }
        
        Ok(())
    }

    // verify a local user password for a user. This is used for local authentication.
    // the method will return the User's object if the username/password combination is valid, false otherwise.
    pub async fn verify_user_local_password(
        &self,
        competition_name: &str,
        username: &str,
        password: &str,
    ) -> Result<Option<User>> {
        let password_hashes_key = self.competition_key(competition_name, "users:password_hashes");

        if let Some(hashed_password) = self
            .redis_hget::<_, _, String>(&password_hashes_key, username)
            .await?
        {
            let hashed_password = argon2::password_hash::PasswordHash::new(&hashed_password)
                .map_err(|e| anyhow::anyhow!("Failed to parse hashed password: {}", e))?;
            let hasher = argon2::Argon2::default();

            if hasher
                .verify_password(password.as_bytes(), &hashed_password)
                .is_ok()
            {
                let user_data_key = self.competition_key(competition_name, "user_data");
                if let Some(user_data_str) = self
                    .redis_hget::<_, _, String>(&user_data_key, username)
                    .await?
                {
                    return Ok(User::from_redis_format(&user_data_str));
                }
            }
        }
        Ok(None)
    }

    // Get all users for a team
    pub async fn get_team_users(
        &self,
        competition_name: &str,
        team_name: &str,
    ) -> Result<Vec<User>> {
        let mut conn = self
            .client
            .get_multiplexed_tokio_connection()
            .await
            .context("Failed to connect to Redis")?;

        let team_users_key = format!("{}:{}:users", competition_name, team_name);
        let users: Vec<String> = redis::cmd("SMEMBERS")
            .arg(&team_users_key)
            .query_async(&mut conn)
            .await
            .context("Failed to get team users")?;

        let mut result = Vec::new();
        for user_name in users {
            // use redis.get_user_data to get the user data
            if let Ok(Some(userdata)) = self.get_user(competition_name, &user_name).await {
                result.push(userdata);
            }
        }

        Ok(result)
    }

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

    // get the global competition state atomically. If the state is not set, will insert a default state (Unstarted).
    pub async fn get_competition_state(&self, competition_name: &str) -> Result<CompetitionState> {
        let mut conn = self
            .client
            .get_multiplexed_tokio_connection()
            .await
            .context("Failed to connect to Redis")?;

        // Key for competition state
        let key = format!("{}:state", competition_name);

        // Try to get the state
        let state: Option<String> = redis::cmd("HGET")
            .arg(&key)
            .arg("state")
            .query_async(&mut conn)
            .await
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
                    .query_async(&mut conn)
                    .await
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
                                        .query_async(&mut conn)
                                        .await
                                        .context(
                                            "Failed to update competition state to finished",
                                        )?;
                                    // Optionally publish the finished event
                                    let _: () = redis::cmd("PUBLISH")
                                        .arg(format!("{}:events", competition_name))
                                        .arg(serde_yaml::to_string(&state).context(
                                            "Failed to serialize finished state for publish",
                                        )?)
                                        .query_async(&mut conn)
                                        .await
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
    pub async fn start_competition(
        &self,
        competition_name: &str,
        duration: Option<u64>,
    ) -> Result<()> {
        let mut conn = self
            .client
            .get_multiplexed_tokio_connection()
            .await
            .context("Failed to connect to Redis")?;
        // use get_competition_state to check current state
        let current_state = self.get_competition_state(competition_name).await?;
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
                    .query_async(&mut conn)
                    .await
                    .context("Failed to start competition")?;
                // publish the start event to a channel
                let _: () = redis::cmd("PUBLISH")
                    .arg(format!("{}:events", competition_name))
                    .arg(
                        serde_yaml::to_string(&new_state)
                            .context("Failed to serialize competition state for publish")?,
                    )
                    .query_async(&mut conn)
                    .await
                    .context("Failed to publish competition start event")?;
                Ok(())
            }
            CompetitionStatus::Finished => Err(anyhow::anyhow!("Competition has already finished")),
        }
    }

    // Ends the competition. Returns an error if the competition is not active.
    pub async fn end_competition(&self, competition_name: &str) -> Result<()> {
        let mut conn = self
            .client
            .get_multiplexed_tokio_connection()
            .await
            .context("Failed to connect to Redis")?;
        // use get_competition_state to check current state
        let current_state = self.get_competition_state(competition_name).await?;
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
                    .query_async(&mut conn)
                    .await
                    .context("Failed to end competition")?;
                // publish the end event to a channel
                let _: () = redis::cmd("PUBLISH")
                    .arg(format!("{}:events", competition_name))
                    .arg(
                        serde_yaml::to_string(&new_state)
                            .context("Failed to serialize competition state for publish")?,
                    )
                    .query_async(&mut conn)
                    .await
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

    pub async fn generate_new_flag(
        &self,
        competition_name: &str,
        team_name: &str,
        flag_check_name: &str,
    ) -> Result<String> {
        let key = format!(
            "{}:{}:{}:flags",
            competition_name, team_name, flag_check_name
        );
        let value = format!(
            "{}{{{}}}",
            competition_name,
            Self::generate_lowercase_string(8)
        );
        self.redis_sadd(&key, &value).await?;
        Ok(value)
    }

    pub async fn redeem_flag(
        &self,
        competition_name: &str,
        team_name: &str,
        team_id: u64,
        flag: &str,
        flag_check: &FlagCheck,
    ) -> Result<bool> {
        let mut conn = self
            .client
            .get_multiplexed_tokio_connection()
            .await
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
            .query_async(&mut conn)
            .await
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
                1, // 1 occurrence for this flag redemption
            )
            .await?;
            // set the current state of the flag check to true
            self.set_check_current_state(
                competition_name,
                team_name,
                &flag_check.name,
                true,
                0, // No failures on successful flag redemption
                vec![event_message],
                (1, 1),     // 1 success out of 1 check
                Vec::new(), // No passing boxes for flag checks
            )
            .await?;
        }

        if exists {
            // Remove the flag from the set
            let _: () = redis::cmd("SREM")
                .arg(&key)
                .arg(flag)
                .query_async(&mut conn)
                .await
                .context("Failed to remove flag from set")?;
            Ok(true) // Flag redeemed successfully
        } else {
            Ok(false) // Flag does not exist
        }
    }

    pub async fn set_check_current_state(
        &self,
        competition_name: &str,
        team_name: &str,
        check_name_or_flag_check_name: &str,
        success: bool,
        number_of_failures: u64,
        messages: Vec<String>,
        success_fraction: (u64, u64), // fraction of successful checks over total checks
        passing_boxes: Vec<String>,
    ) -> Result<()> {
        let key = self.team_key(competition_name, team_name, "current_state");
        let state = CheckCurrentState {
            success,
            number_of_failures,
            message: messages,
            success_fraction,
            passing_boxes,
        };
        let status = Self::serialize_to_yaml(&state)?;
        self.redis_hset(&key, check_name_or_flag_check_name, status)
            .await
    }

    pub async fn get_check_current_state(
        &self,
        competition_name: &str,
        team_name: &str,
        check_name_or_flag_check_name: &str,
    ) -> Result<Option<CheckCurrentState>> {
        let key = self.team_key(competition_name, team_name, "current_state");

        if let Some(state_str) = self
            .redis_hget::<_, _, String>(&key, check_name_or_flag_check_name)
            .await?
        {
            match Self::deserialize_from_yaml(&state_str) {
                Ok(parsed) => return Ok(Some(parsed)),
                Err(e) => {
                    return Err(anyhow::anyhow!(
                        "Invalid state format (YAML): {}: {}",
                        state_str,
                        e
                    ));
                }
            }
        }

        Ok(Some(CheckCurrentState {
            success: false,
            number_of_failures: 0,
            message: Vec::from(["Unsolved".to_string()]),
            success_fraction: (0, 0),
            passing_boxes: Vec::new(),
        }))
    }

    // Get a specific user by username and find their team
    pub async fn get_user(&self, competition_name: &str, username: &str) -> Result<Option<User>> {
        let users_key = self.competition_key(competition_name, "users");
        let user_data_key = self.competition_key(competition_name, "user_data");

        let mut conn = self.get_connection().await?;
        let user_exists: bool = redis::cmd("SISMEMBER")
            .arg(&users_key)
            .arg(username)
            .query_async(&mut conn)
            .await
            .context("Failed to check if user exists")?;

        if user_exists {
            if let Some(data) = self
                .redis_hget::<_, _, String>(&user_data_key, username)
                .await?
            {
                return Ok(User::from_redis_format(&data));
            }
        }
        Ok(None)
    }

    /// Get the score for a team for a specific check up to a given timestamp (inclusive).
    /// Returns the total score (number of successful events * check points).
    pub async fn get_number_of_successful_checks_at_time(
        &self,
        competition_name: &str,
        team_id: u64,
        check_name: &str,
        timestamp: i64,
    ) -> Result<i64> {
        let mut conn = self
            .client
            .get_multiplexed_tokio_connection()
            .await
            .context("Failed to connect to Redis")?;
        // the key name
        let key = format!("{}:{}:{}", competition_name, team_id, check_name);
        // Get the number of events for this team/check up to the timestamp
        let count: i64 = redis::cmd("ZCOUNT")
            .arg(&key)
            .arg("-inf")
            .arg(timestamp)
            .query_async(&mut conn)
            .await
            .context("Failed to get team score by check at time")?;
        // Try to get the check points from the check or flag_check (not available here, so just return count)
        Ok(count)
    }
    pub fn get_number_of_successful_checks_at_times(
        &self,
        competition_name: &str,
        team_id: u64,
        check_name: &str,
        timestamps: impl IntoIterator<Item = i64> + Clone,
    ) -> Result<Vec<i64>> {
        let mut conn = self
            .client
            .get_connection()
            .context("Failed to connect to Redis")?;
        // the key name
        let key = format!("{}:{}:{}", competition_name, team_id, check_name);
        // Get the number of events for this team/check at each timestamp
        Ok(redis::transaction(
            &mut conn,
            &[key.clone()],
            |con, pipe| {
                for timestamp in timestamps.clone() {
                    pipe.zcount(&key, "-inf", timestamp);
                }
                pipe.query(con)
            },
        )?)
    }
}
