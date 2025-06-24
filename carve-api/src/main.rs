use actix_cors::Cors;
use actix_session::{storage::CookieSessionStore, Session, SessionMiddleware};
use actix_web::cookie::{Cookie, Key};
use actix_web::middleware::{self, TrailingSlash};
use actix_web::{
    get, middleware::Logger, web, App, HttpResponse, HttpServer, Responder, Result as ActixResult,
};
use carve::redis_manager::{self, User};
use carve::{
    config::{AppConfig, Competition},
    redis_manager::RedisManager,
};
use chrono::{DateTime, Utc};
use env_logger::Env;
use reqwest;
use oauth2::{basic::*, PkceCodeVerifier, TokenResponse};
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge, RedirectUrl,
    Scope, TokenUrl,
};
use std::error::Error;
mod auth;
mod types;
mod teams;
mod users;
mod boxes;
mod admin;

pub use boxes::get_boxes;
pub use boxes::get_box;
pub use boxes::get_box_default_creds;

// API Handlers
#[get("/competition")]
async fn get_competition(competition: web::Data<Competition>, redis: web::Data<RedisManager>) -> ActixResult<impl Responder> {
    match redis.get_competition_state(&competition.name) {
        Ok(state) => {
           Ok(HttpResponse::Ok().json(state))
        }
        Err(_) => {
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to retrieve competition state"
            })));
        }
    }
}

#[get("/score")]
async fn get_score(
    query: web::Query<types::ScoreQuery>,
    competition: web::Data<Competition>,
    redis: web::Data<RedisManager>,
) -> ActixResult<impl Responder> {
    let end_time = query
        .end_date
        .unwrap_or_else(|| {
            match redis.get_competition_state(&competition.name) {
                Ok(state) => {
                    match state.end_time {
                        Some(end) => end,
                        None => Utc::now(),
                    }
                }
                Err(_) => Utc::now(),
            }
        })
        .timestamp();
    let start_time = query
        .start_date
        .unwrap_or_else(|| {
            match redis.get_competition_state(&competition.name) {
                Ok(state) => {
                    match state.start_time {
                        Some(start) => start,
                        None => Utc::now() - chrono::Duration::days(1),
                    }
                }
                Err(_) => Utc::now() - chrono::Duration::days(1),
            }
        })
        .timestamp();
    let mut scores = Vec::new();

    // If team_id is specified, filter by team
    let teams_to_check: Vec<_> = if let Some(team_id) = query.team_id {
        if team_id as usize > competition.teams.len() || team_id == 0 {
            return Ok(HttpResponse::NotFound().json(serde_json::json!({
                "error": "Team not found"
            })));
        }
        vec![(
            team_id,
            competition.teams[team_id as usize - 1].name.clone(),
        )]
    } else {
        competition
            .teams
            .iter()
            .enumerate()
            .map(|(i, team)| (i as u64 + 1, team.name.clone()))
            .collect()
    };

    // If scoring_check is specified, filter by check
    let checks_to_check: Vec<_> = if let Some(ref check_name) = query.scoring_check {
        if let Some(check) = competition.checks.iter().find(|c| c.name == *check_name) {
            vec![check.clone()]
        } else {
            return Ok(HttpResponse::NotFound().json(serde_json::json!({
                "error": "Scoring check not found"
            })));
        }
    } else {
        competition.checks.clone()
    };

    // Get score events from Redis
    println!("Fetching score events for teams: {:?} and checks: {:?} at time range: {} to {}", 
        teams_to_check, checks_to_check, start_time, end_time);
    for (team_id, team_name) in teams_to_check {
        for check in &checks_to_check {
            match redis.get_team_score_check_events(
                &&competition.name,
                competition.get_team_id_from_name(&team_name).unwrap_or(0),
                &check.name,
                start_time,
                end_time,
            ) {
                Ok(events) => {
                    for (event, timestamp) in events {
                        scores.push(carve::redis_manager::ScoreEvent {
                            team_id,
                            score_event_type : check.name.clone(),
                            box_name: event.box_name.clone(),
                            timestamp: timestamp,
                            message: event.message,
                        });
                    }
                }
                Err(_) => {
                    // Continue even if Redis query fails for this team/check combination
                }
            }
        }
    }

    Ok(HttpResponse::Ok().json(scores))
}

#[get("/leaderboard")]
async fn get_leaderboard(
    competition: web::Data<Competition>,
    redis: web::Data<RedisManager>,
) -> ActixResult<impl Responder> {
    let mut leaderboard_entries = Vec::new();

    for (team_idx, team) in competition.teams.iter().enumerate() {
        let mut total_score = 0i64;

        // Calculate total score for this team across all checks
        for check in &competition.checks {
            match redis.get_team_score_by_check(
                &competition.name,
                competition.get_team_id_from_name(&team.name).unwrap_or(0),
                &check.name,
                check.points as i64,
            ) {
                Ok(score) => total_score += score,
                Err(_) => {
                    // Continue even if Redis query fails for this check
                }
            }
        }

        leaderboard_entries.push(types::LeaderboardEntry {
            team_id: team_idx as u64 + 1,
            team_name: team.name.clone(),
            score: total_score,
            rank: 0, // Will be set after sorting
        });
    }

    // Sort by score (descending) and assign ranks
    leaderboard_entries.sort_by(|a, b| b.score.cmp(&a.score));
    for (idx, entry) in leaderboard_entries.iter_mut().enumerate() {
        entry.rank = idx as u64 + 1;
    }

    let response = types::LeaderboardResponse {
        teams: leaderboard_entries,
    };

    Ok(HttpResponse::Ok().json(response))
}

#[get("/checks")]
async fn get_checks(competition: web::Data<Competition>) -> ActixResult<impl Responder> {
    let checks: Vec<types::CheckResponse> = competition
        .checks
        .iter()
        .map(|check| types::CheckResponse {
            name: check.name.clone(),
            points: check.points,
        })
        .collect();

    Ok(HttpResponse::Ok().json(checks))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    let competition_name =
        std::env::var("COMPETITION_NAME").unwrap_or_else(|_| "default".to_string());
    let config = AppConfig::new().expect("Failed to load configuration");
    let competition = config
        .competitions
        .iter()
        .find(|c| c.name == competition_name)
        .expect("Competition not found in configuration");
    let competition_clone = competition.clone();

    //read the SECRET_KEY from environment variable
    let secret_key = std::env::var("SECRET_KEY").expect("SECRET_KEY not set");
    let secret_key = Key::from(secret_key.as_bytes());
    println!("Starting server for competition: {}", competition.name);

    // get client_id and client_secret from environment variables
    let client_id = std::env::var("OAUTH2_CLIENT_ID").expect("OAUTH2_CLIENT_ID not set");
    let client_secret =
        std::env::var("OAUTH2_CLIENT_SECRET").expect("OAUTH2_CLIENT_SECRET not set");
    let auth_url = std::env::var("OAUTH2_AUTH_URL").expect("OAUTH2_AUTH_URL not set");
    let token_url = std::env::var("OAUTH2_TOKEN_URL").expect("OAUTH2_TOKEN_URL not set");
    let redirect_url = std::env::var("OAUTH2_REDIRECT_URL").expect("OAUTH2_REDIRECT_URL not set");

    let client: types::OauthClient = BasicClient::new(ClientId::new(client_id))
        .set_client_secret(ClientSecret::new(client_secret))
        .set_auth_uri(AuthUrl::new(auth_url).expect("Invalid auth URL"))
        .set_token_uri(TokenUrl::new(token_url).expect("Invalid token URL"))
        .set_redirect_uri(RedirectUrl::new(redirect_url).expect("Invalid redirect URL"));

    let redis_manager = RedisManager::new(&competition.redis).expect("Failed to connect to Redis");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(competition_clone.clone()))
            .app_data(web::Data::new(redis_manager.clone()))
            .app_data(web::Data::new(client.clone()))
            .wrap(Logger::default().log_level(log::Level::Debug))
            .wrap(middleware::NormalizePath::trim())
            .wrap(SessionMiddleware::new(
                CookieSessionStore::default(),
                secret_key.clone(),
            ))
            .service(
                web::scope("/api/v1")
                    .service(
                        web::scope("/competition")
                            .guard(auth::validate_session)
                            .service(get_competition)
                            .service(get_score)
                            .service(get_leaderboard)
                            .service(get_boxes)
                            .service(get_box)
                            .service(get_box_default_creds)
                            .service(get_checks)
                            .service(teams::get_team)
                            .service(teams::get_teams)
                            .service(teams::get_team_console_code)
                            .service(users::get_user)
                            .service(users::switch_team)
                            .service(users::generate_join_code)
                    )
                    .service(
                        web::scope("/oauth2")
                            .wrap(Cors::permissive())
                            .service(auth::get_oauth2_redirect_url)
                            .service(auth::oauth2_callback)
                            .service(auth::logout)
                    )
                    .service(
                        web::scope("/admin")
                            .guard(auth::validate_admin_session)
                            .service(admin::start_competition)
                            .service(admin::end_competition)
                    )
            )
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
