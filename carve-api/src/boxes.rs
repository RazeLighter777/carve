// Boxes-related API handlers

use actix_session::Session;
use actix_web::{get, web, HttpResponse, Responder, Result as ActixResult};
use crate::types;
use carve::config::Competition;
use carve::redis_manager::RedisManager;
use std::process::Stdio;
use tokio::process::Command;

// Helper function to resolve IP address using dig
pub async fn resolve_box_ip(box_name: &str, vtep_host: &str) -> Option<String> {
    let output = Command::new("dig")
        .arg(box_name)
        .arg(&format!("@{}", vtep_host))
        .arg("+short")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await;
    let output = output;

    match output {
        Ok(output) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let ip = stdout.trim();

                // Validate IPv4 address format
                if !ip.is_empty() && is_valid_ipv4(ip) {
                    Some(ip.to_string())
                } else {
                    None
                }
            } else {
                None
            }
        }
        Err(_) => None,
    }
}

// Helper function to validate IPv4 address format
pub fn is_valid_ipv4(ip: &str) -> bool {
    let parts: Vec<&str> = ip.split('.').collect();
    if parts.len() != 4 {
        return false;
    }

    for part in parts {
        if let Ok(_) = part.parse::<u8>() {
            // Valid if it's a number between 0-255
            if part.len() > 1 && part.starts_with('0') {
                return false; // No leading zeros allowed
            }
        } else {
            return false;
        }
    }

    true
}

#[get("/boxes")]
pub async fn get_boxes(
    query: web::Query<types::BoxesQuery>,
    competition: web::Data<Competition>,
) -> ActixResult<impl Responder> {
    // ...existing code from main.rs...
    let team_id = query.team_id as usize;
    if team_id == 0 || team_id > competition.teams.len() {
        return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "Team not found"
        })));
    }

    let team_name = &competition.teams[team_id - 1].name;

    // Generate box names based on team name and available boxes
    let boxes: Vec<types::BoxInfo> = competition
        .boxes
        .iter()
        .map(|box_config| types::BoxInfo {
            name: format!(
                "{}.{}.{}.local",
                box_config.name,
                team_name.to_lowercase(),
                competition.name.to_lowercase()
            ),
        })
        .collect();

    Ok(HttpResponse::Ok().json(boxes))
}

#[get("/box")]
pub async fn get_box(
    query: web::Query<types::BoxQuery>,
    competition: web::Data<Competition>,
) -> ActixResult<impl Responder> {
    // ...existing code from main.rs...
    let parts: Vec<&str> = query.name.split('.').collect();
    if parts.len() < 3 {
        return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "Box not found"
        })));
    }

    let box_type = parts[0];
    let team_name = parts[1];

    // Verify the box type exists in configuration
    let box_exists = competition.boxes.iter().any(|b| b.name == box_type);

    // Verify the team exists
    let team_exists = competition
        .teams
        .iter()
        .any(|t| t.name.to_lowercase() == team_name);

    if !box_exists || !team_exists {
        return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "Box not found"
        })));
    }

    // Resolve IP address using dig if vtep_host is configured
    let ip_address = if let Some(ref vtep_host) = competition.vtep_host {
        resolve_box_ip(&query.name, vtep_host)
            .await
            .unwrap_or_else(|| "0.0.0.0".to_string()) // Fallback if resolution fails
    } else {
        "192.168.1.100".to_string() // Fallback if no vtep_host configured
    };

    let response = types::BoxDetailResponse {
        name: query.name.clone(),
        ip_address,
        status: "active".to_string(),
    };

    Ok(HttpResponse::Ok().json(response))
}

#[get("box/creds")]
pub async fn get_box_default_creds(
    query: web::Query<types::BoxQuery>,
    competition: web::Data<Competition>,
    redis: web::Data<RedisManager>,
    session : Session,
) -> ActixResult<impl Responder> {
    // ...existing code from main.rs...
    let parts: Vec<&str> = query.name.split('.').collect();
    if parts.len() < 3 {
        return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "Creds not set"
        })));
    }

    let box_type = parts[0];
    let team_name = parts[1];

    //verify the user belongs to the team
    if let Some(session_team_name) = session.get::<String>("team_name")? {
        if session_team_name != team_name {
            return Ok(HttpResponse::Forbidden().json(serde_json::json!({
                "error": "You do not have permission to access this box"
            })));
        }
    } else {
        return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
            "error": "You must be logged in to access this box"
        })));
    }

    // Try to get credentials from Redis
    match redis.read_box_credentials(&competition.name, team_name, box_type) {
        Ok(Some((username, password))) => {
            let response = types::BoxCredentialsResponse {
                name: query.name.clone(),
                username,
                password,
            };
            Ok(HttpResponse::Ok().json(response))
        }
        Ok(None) => Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "Creds not set"
        }))),
        Err(_) => Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "Creds not set"
        }))),
    }
}

#[get("/box/send_command")]
pub async fn send_box_command(
    query: web::Query<types::BoxCommandQuery>,
    competition: web::Data<Competition>,
    redis: web::Data<RedisManager>,
    session : Session,
) -> ActixResult<impl Responder> {
    let parts: Vec<&str> = query.box_name.split('.').collect();
    if parts.len() < 3 {
        return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "Box not found"
        })));
    }

    let box_type = parts[0];
    let team_name = parts[1];
    let command = query.command.clone();

    // Verify the user belongs to the team
    if let Some(session_team_name) = session.get::<String>("team_name")? {
        if session_team_name != team_name {
            return Ok(HttpResponse::Forbidden().json(serde_json::json!({
                "error": "You do not have permission to access this box"
            })));
        }
    } else {
        return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
            "error": "You must be logged in to access this box"
        })));
    }
    // Send command to Redis
    match redis.send_qemu_event(&competition.name, &team_name, box_type, command) {
        Ok(_) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "status": "Command sent successfully"
        }))),
        Err(_) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Failed to send command"
        }))),
    }
}