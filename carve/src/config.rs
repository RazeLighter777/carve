// Configuration logic moved from canary/src/config.rs
use anyhow::Result;
use config::{Config, File};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize, Clone)]
pub struct RedisConfig {
    pub host: String,
    pub port: u16,
    pub db: u8,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Box {
    pub name: String,
    pub labels: String,
    pub hostname: String,
    pub cores: Option<u32>, // Optional number of CPU cores
    pub ram_mb: Option<u32>, // Optional RAM in MB
}

#[derive(Debug, Deserialize, Clone)]
pub struct Team {
    pub name: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct HttpCheckSpec {
    pub url: String,
    pub code: u16,
    pub regex: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct IcmpCheckSpec {
    pub code: u8,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SshCheckSpec {
    pub port: u16,
    pub username: String,
    pub password: Option<String>,
    pub key_path: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum CheckSpec {
    #[serde(rename = "http")]
    Http(HttpCheckSpec),
    #[serde(rename = "icmp")]
    Icmp(IcmpCheckSpec),
    #[serde(rename = "ssh")]
    Ssh(SshCheckSpec),
}

#[derive(Debug, Deserialize, Clone)]
pub struct Check {
    pub name: String,
    pub interval: u64,
    pub points: u32,
    pub label_selector: Option<HashMap<String, String>>,
    #[serde(rename = "labelSelector")]
    pub label_selector_alt: Option<HashMap<String, String>>,
    pub spec: CheckSpec,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Competition {
    pub name: String,
    pub redis: RedisConfig,
    pub cidr: Option<String>, // Add this field for VTEP
    pub vtep_host: Option<String>, // <-- Add this line for VTEP host
    pub boxes: Vec<Box>,
    pub teams: Vec<Team>,
    pub checks: Vec<Check>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub competitions: Vec<Competition>,
}

impl AppConfig {
    pub fn new() -> Result<Self> {
        let config = Config::builder()
            .add_source(File::with_name("competition.yaml").required(false))
            .add_source(File::with_name("/app/competition.yaml").required(false))
            .add_source(File::with_name("/config/competition.yaml").required(false))
            .build()?;

        let app_config: AppConfig = config.try_deserialize()?;
        Ok(app_config)
    }
}
