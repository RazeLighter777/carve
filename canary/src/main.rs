mod check;
mod scheduler;

use actix_web::{App, HttpResponse, HttpServer, Responder, get, web};
use anyhow::Result;
use env_logger::Env;
use log::{error, info};
use serde::Serialize;
use std::sync::Arc;

use crate::scheduler::Scheduler;
use carve::config::AppConfig;
use carve::redis_manager::RedisManager;

struct AppState {
    redis_managers: Vec<Arc<RedisManager>>,
}

#[derive(Serialize)]
struct ScoreResponse {
    competition: String,
    team: String,
    score: i64,
}

#[derive(Serialize)]
struct CheckScore {
    check: String,
    score: i64,
}

#[get("/api/health")]
async fn health_check(data: web::Data<AppState>) -> impl Responder {
    for (i, redis_manager) in data.redis_managers.iter().enumerate() {
        if let Err(e) = redis_manager.health_check() {
            error!("Redis connection {} failedthe health check: {}", i, e);
            return HttpResponse::InternalServerError()
                .body(format!("Redis connection failed: {}", e));
        }
    }

    HttpResponse::Ok().body("Healthy")
}

#[get("/api/score/{competition}/{team}")]
async fn get_team_score(
    path: web::Path<(String, String)>,
    data: web::Data<AppState>,
) -> impl Responder {
    let (competition_name, team_name) = path.into_inner();
    let config = AppConfig::new().expect("Failed to load configuration");

    // get the competition from the config
    let competition = config
        .competitions
        .iter()
        .find(|c| c.name == competition_name)
        .expect("Competition not found in configuration");
    if let Some(team_id) = competition.get_team_id_from_name(&team_name) {
        info!(
            "Getting score for team {} (ID: {}) in competition {}",
            team_name, team_id, competition_name
        );
        for redis_manager in &data.redis_managers {
            // iterate over the team's checks
            let mut total_score = 0;
            let mut scores = Vec::new();
            for check in &competition.checks {
                // Try to get the score using this manager
                match redis_manager.get_team_score_by_check(
                    &competition_name,
                    team_id,
                    &check.name,
                    check.points as i64,
                ) {
                    Ok(score) => {
                        total_score += score;
                        scores.push(CheckScore {
                            check: check.name.clone(),
                            score,
                        });
                    }
                    Err(e) => {
                        error!(
                            "Failed to get score for team {} on check {}: {}",
                            team_name, check.name, e
                        );
                        // Continue trying with other managers
                    }
                }
            }
            if total_score > 0 {
                return HttpResponse::Ok().json(ScoreResponse {
                    competition: competition_name,
                    team: team_name,
                    score: total_score,
                });
            }
        }
    } else {
        return HttpResponse::NotFound().body(format!(
            "Team {} not found in competition {}",
            team_name, competition_name
        ));
    }

    // If we got here, we didn't find a matching competition or there was an error
    HttpResponse::NotFound().body(format!(
        "Could not find score for team {} in competition {}",
        team_name, competition_name
    ))
}

#[get("/api/score/{competition}/{team}/{check}")]
async fn get_team_score_by_check(
    path: web::Path<(String, String, String)>,
    data: web::Data<AppState>,
) -> impl Responder {
    let (competition_name, team_name, check_name) = path.into_inner();
    let config = AppConfig::new().expect("Failed to load configuration");
    // get the competition from the config
    let competition = config
        .competitions
        .iter()
        .find(|c| c.name == competition_name)
        .expect("Competition not found in configuration");
    // get the check from the competition
    let check = competition
        .checks
        .iter()
        .find(|c| c.name == check_name)
        .expect("Check not found in competition");
    // Get the points for this check
    let check_points = check.points as i64;
    // Get the team ID from the competition
    if let Some(team_id) = competition.get_team_id_from_name(&team_name) {
        info!(
            "Getting score for team {} (ID: {}) on check {}",
            team_name, team_id, check_name
        );
        // Find the Redis manager for this competition
        for redis_manager in &data.redis_managers {
            // Try to get the score using this manager
            match redis_manager.get_team_score_by_check(
                &competition_name,
                team_id,
                &check_name,
                check_points,
            ) {
                Ok(score) => {
                    return HttpResponse::Ok().json(ScoreResponse {
                        competition: competition_name.clone(),
                        team: team_name.clone(),
                        score,
                    });
                }
                Err(e) => {
                    error!(
                        "Failed to get score for team {} on check {}: {}",
                        team_name, check_name, e
                    );
                    // Continue trying with other managers
                }
            }
        }
    } else {
        return HttpResponse::NotFound().body(format!(
            "Team {} not found in competition {} when trying to get score for check {}",
            team_name, competition_name, check_name
        ));
    }

    // If we got here, we didn't find a matching competition or there was an error
    HttpResponse::NotFound().body(format!(
        "Could not find score for team {} on check {} in competition {}",
        team_name, check_name, competition_name
    ))
}
#[actix_web::main]
async fn main() -> Result<()> {
    // Initialize logger
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    // Load configuration
    let config = match AppConfig::new() {
        Ok(cfg) => cfg,
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            return Err(e);
        }
    };

    info!(
        "Loaded configuration with {} competitions",
        config.competitions.len()
    );

    // Initialize Redis managers for each competition
    let mut redis_managers = Vec::new();

    for competition in &config.competitions {
        let redis_manager = match RedisManager::new(&competition.redis) {
            Ok(manager) => Arc::new(manager),
            Err(e) => {
                error!(
                    "Failed to create Redis manager for {}: {}",
                    competition.name, e
                );
                return Err(e);
            }
        };

        info!(
            "Initialized Redis manager for competition: {}",
            competition.name
        );
        redis_managers.push(redis_manager.clone());

        // Create and run scheduler for this competition
        let scheduler = Scheduler::new(competition.clone(), redis_manager);
        scheduler.run().await;

        info!("Started scheduler for competition: {}", competition.name);
    }

    // Start the web server
    let app_state = web::Data::new(AppState { redis_managers });

    info!("Starting HTTP server on 0.0.0.0:8080");
    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .service(health_check)
            .service(get_team_score)
            .service(get_team_score_by_check)
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await?;

    Ok(())
}
