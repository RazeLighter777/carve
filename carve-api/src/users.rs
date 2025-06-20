// User-related API handlers

use crate::types;
use actix_session::Session;
use actix_web::{get, web, HttpResponse, Responder, Result as ActixResult};
use carve::config::Competition;
use carve::redis_manager::{RedisManager, User};

#[get("/user")]
pub async fn get_user(
    query: web::Query<types::UserQuery>,
    competition: web::Data<Competition>,
    redis: web::Data<RedisManager>,
) -> ActixResult<impl Responder> {
    // ...existing code from main.rs...
    let username = query.username.clone();
    // Get user from Redis
    match redis.get_user(&competition.name, &username) {
        Ok(Some(user)) => {
            let team_id = if let Some(ref team_name) = user.team_name {
                competition
                    .teams
                    .iter()
                    .position(|t| t.name == *team_name)
                    .map(|idx| idx as u64 + 1)
            } else {
                None
            };

            let response = types::UserResponse {
                name: user.username,
                email: user.email,
                team_id,
            };
            Ok(HttpResponse::Ok().json(response))
        }
        Ok(None) => Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "User not found"
        }))),
        Err(_) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Failed to retrieve user"
        }))),
    }
}

#[get("/switch_team")]
pub async fn switch_team(
    query: web::Query<types::SwitchTeamQuery>,
    competition: web::Data<Competition>,
    redis: web::Data<RedisManager>,
    session: Session
) -> ActixResult<impl Responder> {
    //get the user's name from the session, return an error if not found
    if let Some(username) = session.get::<String>("username").unwrap_or(None) {
        if username.is_empty() {
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "error": "User not logged in"
            })));
        }
        // call redis manager's check_team_join_code to get team id (if the code is valid)
        let team_name = match redis.check_team_join_code(&competition.name, query.team_join_code) {
            Ok(Some(id)) => id,
            Ok(None) => {
                return Ok(HttpResponse::NotFound().json(serde_json::json!({
                    "error": "Team not found"
                })))
            }
            Err(_) => {
                return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "Failed to check team join code"
                })))
            }
        };
        // call redis manager's register_user to switch the user to the new team
        match redis.register_user(
            &competition.name,
            &User {
                username: username,
                email: session
                    .get::<String>("email")
                    .unwrap_or(None)
                    .unwrap_or_default(),
                team_name: Some(team_name.clone()),
                //check is is_admin is set in session, if not set to false
                is_admin: session
                    .get::<bool>("is_admin")
                    .unwrap_or(Some(false))
                    .unwrap_or(false),
            },
            Some(&team_name),
        ) {
            Ok(_) => Ok(HttpResponse::Ok().json(serde_json::json!({
                "message": "Switched team successfully",
                "team_name": team_name,
            }))),
            Err(_) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to switch team"
            }))),
        }
    } else {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "User not logged in"
        })));
    }
}

#[get("/generate_join_code")]
pub async fn generate_join_code(
    session: Session,
    competition: web::Data<Competition>,
    redis: web::Data<RedisManager>,
) -> ActixResult<impl Responder> {
    // get the team the user is currently in. If the user is not in a team, return an error
    let team_name = session
        .get::<String>("team_name")
        .unwrap_or(None)
        .unwrap_or_default();
    if team_name.is_empty() {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "User is not in a team"
        })));
    }
    // generate a join code for the team
    match redis.generate_team_join_code(&competition.name, &team_name) {
        Ok(join_code) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "code": join_code,
        }))),
        Err(_) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Failed to generate join code"
        }))),
    }
}
