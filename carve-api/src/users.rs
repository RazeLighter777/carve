// User-related API handlers

use crate::types;
use actix_session::Session;
use actix_web::{get, web, HttpResponse, Responder, Result as ActixResult};
use actix_ws::AggregatedMessage;
use carve::config::Competition;
use carve::redis_manager::{RedisManager, User};
use futures::StreamExt as _;

#[get("/user")]
pub async fn get_user(
    query: web::Query<types::UserQuery>,
    competition: web::Data<Competition>,
    redis: web::Data<RedisManager>,
) -> ActixResult<impl Responder> {
    // ...existing code from main.rs...
    let username = query.username.clone();
    // Get user from Redis
    match redis.get_user(&competition.name, &username).await {
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
    session: Session,
) -> ActixResult<impl Responder> {
    //get the user's name from the session, return an error if not found
    if let Some(username) = session.get::<String>("username").unwrap_or(None) {
        if username.is_empty() {
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "error": "User not logged in"
            })));
        }
        // call redis manager's check_team_join_code to get team id (if the code is valid)
        let team_name = match redis
            .check_team_join_code(&competition.name, query.team_join_code)
            .await
        {
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
        match redis
            .register_user(
                &competition.name,
                &User {
                    username,
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
                    identity_sources: vec![],
                },
                Some(&team_name),
            )
            .await
        {
            Ok(_) => Ok(HttpResponse::Ok().json(serde_json::json!({
                "message": "Switched team successfully",
                "team_name": team_name,
            }))),
            Err(_) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to switch team"
            }))),
        }
    } else {
        Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "User not logged in"
        })))
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
    // if the option config.allow_non_admins_to_generate_join_codes is false, return an error
    if !competition.allow_non_admins_to_generate_join_codes {
        return Ok(HttpResponse::Forbidden().json(serde_json::json!({
            "error": "Non-admins are not allowed to generate join codes"
        })));
    }
    // generate a join code for the team
    match redis
        .generate_team_join_code(&competition.name, &team_name)
        .await
    {
        Ok(join_code) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "code": join_code,
        }))),
        Err(_) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Failed to generate join code"
        }))),
    }
}

#[get("/listen_toasts")]
async fn listen_for_toasts(
    redis : web::Data<RedisManager>,
    req: actix_web::HttpRequest,
    stream: web::Payload,
    subscribe_request: web::Query<types::ToastSubscribeRequest>,
) -> Result<HttpResponse, actix_web::Error> {
    let (res, mut session, stream) = actix_ws::handle(&req, stream)?;

    let mut stream = stream
        .aggregate_continuations()
        // aggregate continuation frames up to 1MiB
        .max_continuation_size(2_usize.pow(20));

    // start task but don't wait for it
    let mut session_clone = session.clone();
    actix_web::rt::spawn(async move {
        while let Ok(msg) = redis.wait_for_next_toast(subscribe_request.user.clone(), subscribe_request.team.clone()).await {
            if let Some(toast) = msg {
                // send the toast notification to the client
                if let Err(e) = session_clone.text(serde_json::to_string(&toast).unwrap_or_default()).await {
                    log::error!("Failed to send toast notification: {}", e);
                    break;
                }
            }
        }
    });
    // start task to respond to ping
    actix_web::rt::spawn(async move {
        while let Some(Ok(msg)) = stream.next().await {
            match msg {
                AggregatedMessage::Ping(msg) => {
                    if let Err(e) = session.pong(&msg).await {
                        log::error!("Failed to send pong response: {}", e);
                        break;
                    }
                }
                _ => {}
            }
        }
    });

    // respond immediately with response connected to WS session
    Ok(res)
}
