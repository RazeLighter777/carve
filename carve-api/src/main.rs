use actix_cors::Cors;
use actix_session::{storage::CookieSessionStore, SessionMiddleware};
use actix_web::cookie::Key;
use actix_web::middleware::{self};
use actix_web::post;
use actix_web::{
    get, middleware::Logger, web, App, HttpResponse, HttpServer, Responder, Result as ActixResult,
};
use anyhow::Context;
use carve::config::{Check, FlagCheck};
use carve::{
    config::{AppConfig, Competition},
    redis_manager::RedisManager,
};
use env_logger::Env;
use oauth2::basic::*;
use oauth2::{AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl};
mod admin;
mod auth;
mod boxes;
mod flag;
mod teams;
mod types;
mod users;

pub use boxes::get_box;
pub use boxes::get_box_creds_for_team;
pub use boxes::get_box_default_creds;
pub use boxes::get_boxes;
use rand::distr::SampleString;

// API Handlers
#[get("/competition")]
async fn get_competition(
    competition: web::Data<Competition>,
    redis: web::Data<RedisManager>,
) -> ActixResult<impl Responder> {
    match redis.get_competition_state(&competition.name).await {
        Ok(state) => Ok(HttpResponse::Ok().json(state)),
        Err(_) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Failed to retrieve competition state"
        }))),
    }
}

//returns the score at a given point in time filtered by check
#[post("/scoresat")]
async fn get_scores_at_given_time(
    query: web::Json<types::ScoresAtGivenTimesQuery>,
    competition: web::Data<Competition>,
    redis: web::Data<RedisManager>,
) -> ActixResult<impl Responder> {
    let team_id = query.team_id;
    let at_times = query
        .at_times
        .iter()
        .map(|dt| dt.timestamp())
        .collect::<Vec<_>>();

    // Validate team
    if team_id == 0 || team_id as usize > competition.teams.len() {
        return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "Team not found"
        })));
    }

    // Determine which checks to use
    let checks_to_check: Vec<_> = if let Some(ref check_name) = query.scoring_check {
        if let Some(check) = competition.checks.iter().find(|c| c.name == *check_name) {
            vec![check.name.clone()]
        } else if let Some(flag_check) = competition
            .flag_checks
            .iter()
            .find(|c| c.name == *check_name)
        {
            vec![flag_check.name.clone()]
        } else {
            return Ok(HttpResponse::NotFound().json(serde_json::json!({
                "error": "Scoring check not found"
            })));
        }
    } else {
        let mut all: Vec<String> = competition.checks.iter().map(|c| c.name.clone()).collect();
        all.extend(competition.flag_checks.iter().map(|fc| fc.name.clone()));
        all
    };

    let mut total_score = vec![0i64; at_times.len()];

    for check_name in checks_to_check {
        // For each check, get the score for this team up to at_time
        match redis.get_number_of_successful_checks_at_times(
            &competition.name,
            team_id,
            &check_name,
            at_times.clone(),
        ).await {
            Ok(scores) => {
                for (i, score) in scores.iter().enumerate() {
                    total_score[i] += score;
                }
            }
            Err(_) => {
                // Continue even if Redis query fails for this check
            }
        }
    }

    Ok(HttpResponse::Ok().json(types::ScoresAtGivenTimeResponse {
        scores: total_score,
    }))
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
            match redis
                .get_team_score_by_check(
                    &competition.name,
                    competition.get_team_id_from_name(&team.name).unwrap_or(0),
                    &check.name,
                    check.points as i64,
                )
                .await
            {
                Ok(score) => total_score += score,
                Err(_) => {
                    // Continue even if Redis query fails for this check
                }
            }
        }
        // Also include flag checks in the total score
        for flag_check in &competition.flag_checks {
            match redis
                .get_team_score_by_check(
                    &competition.name,
                    competition.get_team_id_from_name(&team.name).unwrap_or(0),
                    &flag_check.name,
                    flag_check.points as i64,
                )
                .await
            {
                Ok(score) => total_score += score,
                Err(_) => {
                    // Continue even if Redis query fails for this flag check
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
    let _ = redis.set_team_last_known_scores(&competition.name, leaderboard_entries.iter().map(|e| (e.team_name.clone(), e.score)).collect::<Vec<_>>()).await.context("Failed to set team last known scores");

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
    let checks: Vec<Check> = competition.checks.clone();
    let flag_checks: Vec<FlagCheck> = competition.flag_checks.clone();
    // Combine checks and flags into a single response
    let checks = types::CheckResponse {
        checks,
        flag_checks,
    };

    Ok(HttpResponse::Ok().json(checks))
}

#[get("/submit")]
async fn submit_flag(
    session: actix_session::Session,
    query: web::Query<types::RedeemFlagQuery>,
    competition: web::Data<Competition>,
    redis: web::Data<RedisManager>,
) -> ActixResult<impl Responder> {
    // Get username from session
    let username = match session.get::<String>("username").unwrap_or(None) {
        Some(u) => u,
        None => {
            return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "Not logged in"
            })));
        }
    };
    // Get user and team name
    let user = match redis.get_user(&competition.name, &username).await {
        Ok(Some(u)) => u,
        _ => {
            return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "User not found"
            })));
        }
    };
    let team_name = match user.team_name {
        Some(ref t) => t,
        None => {
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "error": "User is not on a team"
            })));
        }
    };
    // Find the flag check
    let flag_check = match competition
        .flag_checks
        .iter()
        .find(|fc| fc.name == query.flag_check_name)
    {
        Some(fc) => fc,
        None => {
            return Ok(HttpResponse::NotFound().json(serde_json::json!({
                "error": "Flag check not found"
            })));
        }
    };
    // Disallow flag submission if competition is not Active (fetch from Redis)
    let state = match redis.get_competition_state(&competition.name).await {
        Ok(s) => s,
        Err(_) => {
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to fetch competition state"
            })));
        }
    };
    if state.status != carve::redis_manager::CompetitionStatus::Active {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Flag submission is only allowed while the competition is active."
        })));
    }
    // Attempt to redeem the flag
    match redis
        .redeem_flag(
            &competition.name,
            team_name,
            competition.get_team_id_from_name(team_name).unwrap_or(0),
            &query.flag,
            flag_check,
        )
        .await
    {
        Ok(true) => Ok(HttpResponse::Ok().json(types::RedeemFlagResponse {
            success: true,
            message: "Flag accepted!".to_string(),
        })),
        Ok(false) => Ok(HttpResponse::BadRequest().json(types::RedeemFlagResponse {
            success: false,
            message: "Incorrect or already redeemed flag.".to_string(),
        })),
        Err(e) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to redeem flag: {}", e)
        }))),
    }
}

pub async fn generate_admin_user_if_not_exists(
    redis: &RedisManager,
    competition: &Competition,
) -> Result<(), String> {
    if let Ok(users) = redis.get_all_users(&competition.name).await {
        if users.iter().any(|u| u.is_admin) {
            return Ok(());
        }
    }
    // If no admin user exists, create one
    let admin_user = carve::redis_manager::User {
        username: "admin".to_string(),
        email: "admin@example.com".to_string(),
        team_name: None,
        is_admin: true,
        identity_sources: vec![carve::redis_manager::IdentitySources::LocalUserPassword],
    };
    redis
        .register_user(&competition.name, &admin_user, None)
        .await
        .expect("Failed to create admin user");
    println!("Admin user created: {}", admin_user.username);
    // generate a password for the admin user
    let mut rng = rand::rng();
    let password = rand::distr::Alphanumeric
        .sample_string(&mut rng, 12)
        .to_string();
    println!("Generated password for admin user: {}", password);
    // Store the password in Redis
    redis
        .set_user_local_password(&competition.name, &admin_user.username, &password)
        .await
        .expect("Failed to set admin user password");

    Ok(())
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
    // if the competition has create_default_admin set to true, generate an admin user
    if competition.create_default_admin {
        generate_admin_user_if_not_exists(&redis_manager, competition)
            .await
            .expect("Failed to generate admin user");
    }

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
                            .service(teams::get_team_check_status)
                            .service(submit_flag)
                            .service(boxes::send_box_restore)
                            .service(get_scores_at_given_time)
                            .service(users::listen_for_toasts)
                    )
                    .service(
                        web::scope("/oauth2")
                            .wrap(Cors::permissive())
                            .service(auth::get_oauth2_redirect_url)
                            .service(auth::oauth2_callback)
                            .service(auth::logout),
                    )
                    .service(
                        web::scope("/auth")
                            .wrap(Cors::permissive())
                            .service(auth::login)
                            .service(auth::register)
                            .service(auth::logout)
                            .service(auth::identity_sources),
                    )
                    .service(
                        web::scope("/admin")
                            .guard(auth::validate_admin_session)
                            .service(admin::start_competition)
                            .service(admin::end_competition)
                            .service(admin::generate_join_code)
                            .service(admin::create_api_key)
                            .service(admin::get_api_keys)
                            .service(admin::delete_api_key)
                            .service(boxes::send_box_snapshot)
                            .service(boxes::get_box_creds_for_team)
                            .service(admin::publish_toast)
                    )
                    .service(
                        web::scope("/internal")
                            .wrap(middleware::from_fn(auth::validate_bearer_token))
                            .service(flag::generate_flag)
                            .service(boxes::get_box)
                            .service(boxes::get_box_creds_for_team),
                    ),
            )
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
