mod check;
mod config;
mod redis_manager;
mod scheduler;

use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use anyhow::Result;
use env_logger::Env;
use log::{error, info};
use std::sync::Arc;

use crate::config::AppConfig;
use crate::redis_manager::RedisManager;
use crate::scheduler::Scheduler;

struct AppState {
    redis_managers: Vec<Arc<RedisManager>>,
}

#[get("/api/health")]
async fn health_check(data: web::Data<AppState>) -> impl Responder {
    for (i, redis_manager) in data.redis_managers.iter().enumerate() {
        if let Err(e) = redis_manager.health_check() {
            error!("Redis connection {} failed health check: {}", i, e);
            return HttpResponse::InternalServerError().body(format!("Redis connection failed: {}", e));
        }
    }
    
    HttpResponse::Ok().body("Healthy")
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
    
    info!("Loaded configuration with {} competitions", config.competitions.len());
    
    // Initialize Redis managers for each competition
    let mut redis_managers = Vec::new();
    
    for competition in &config.competitions {
        let redis_manager = match RedisManager::new(&competition.redis) {
            Ok(manager) => Arc::new(manager),
            Err(e) => {
                error!("Failed to create Redis manager for {}: {}", competition.name, e);
                return Err(e);
            }
        };
        
        info!("Initialized Redis manager for competition: {}", competition.name);
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
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await?;
    
    Ok(())
}
