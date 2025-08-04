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

// Module declarations
mod redis_admin;
mod redis_auth;
mod redis_boxes;
mod redis_competition;
mod redis_flag;
mod redis_teams;
mod redis_users;

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

    pub async fn health_check(&self) -> Result<()> {
        let mut conn = self.get_connection().await?;
        redis::cmd("PING")
            .query_async::<String>(&mut conn)
            .await
            .context("Failed to ping Redis")?;
        Ok(())
    }
}
