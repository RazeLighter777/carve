// Configuration logic moved from canary/src/config.rs
use anyhow::Result;
use config::{Config, File};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::redis_manager::IdentitySources;

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
    pub cores: Option<u32>,  // Optional number of CPU cores
    pub ram_mb: Option<u32>, // Optional RAM in MB
    pub backing_image: String, // Path to the original disk image
}

#[derive(Debug, Deserialize, Clone)]
pub struct Team {
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, PartialOrd)]
pub enum HttpMethods {
    Get,
    Post,
    Put,
    Delete,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct HttpCheckSpec {
    pub url: String,
    pub code: u16,
    pub regex: String,
    pub method: HttpMethods, // HTTP method to use for the check
    pub forms : Option<String>, // Optional forms to submit with the request
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct IcmpCheckSpec {
    pub code: u8,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Hint {
    pub string: String, // Hint text
    pub penalty: u64,   // Points penalty for using this hint
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FlagCheck {
    pub name: String,        // Challenge name. Must be unique.
    pub description: String, // Description of the challenge
    pub points: u64,         // Points awarded for solving the challenge
    pub attempts: u64,       // Number of attempts allowed
    pub box_name: String,    // Name of the box where the flag is located
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SshCheckSpec {
    pub port: u16,
    pub username: String,
    pub password: Option<String>,
    pub key_path: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct NixCheckSpec {
    pub script: String,
    pub packages: Option<Vec<String>>, // Optional list of Nix packages to install
    pub timeout: u64, // Timeout for the check in seconds
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq)]
pub enum RegistrationType {
    OidcOnly,
    Join,
    TeamWithLeastMembers,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
pub enum CheckSpec {
    #[serde(rename = "http")]
    Http(HttpCheckSpec),
    #[serde(rename = "icmp")]
    Icmp(IcmpCheckSpec),
    #[serde(rename = "ssh")]
    Ssh(SshCheckSpec),
    #[serde(rename = "nix")]
    Nix(NixCheckSpec), // Assuming NixCheckSpec is defined elsewhere
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Check {
    pub name: String,
    pub description: String,
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
    pub oidc_provider_name: String, // OIDC provider name
    pub cidr: Option<String>,       // Add this field for VTEP
    pub dns_host: Option<String>,  // <-- Add this line for VTEP host
    pub vtep_host: Option<String>,  // VTEP host
    pub boxes: Vec<Box>,
    pub teams: Vec<Team>,
    pub checks: Vec<Check>,
    pub flag_checks: Vec<FlagCheck>, // Flag checks for the competition
    pub admin_group: Option<String>, // Optional admin group for oidc
    pub description: Option<String>, // Optional description
    pub duration: Option<u64>,       // duration in seconds
    pub registration_type: RegistrationType, // Registration type
    pub identity_sources: Vec<IdentitySources>,
    pub create_default_admin: bool, // Create default admin user
    pub dns_upstream_service : Option<String>, // DNS upstream service for VTEP and carve-novnc-nginx
    pub restore_cooldown: Option<u64>, // Cooldown period for restoring boxes
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

impl Competition {
    pub fn get_team_by_name(&self, team_name: &str) -> Option<&Team> {
        self.teams
            .iter()
            .find(|team| team.name.eq_ignore_ascii_case(team_name))
    }

    pub fn get_team_id_from_name(&self, team_name: &str) -> Option<u64> {
        self.teams
            .iter()
            .position(|team| team.name.eq_ignore_ascii_case(team_name))
            .map(|id| id as u64 + 1)
    }

    pub fn get_team_name_from_id(&self, team_id: u64) -> Option<String> {
        if team_id <= self.teams.len() as u64 {
            Some(self.teams[team_id as usize - 1].name.clone())
        } else {
            None
        }
    }
}
