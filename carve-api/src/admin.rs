// Admin-related functionality for the Carve API

use actix_web::{delete, get, post, web, HttpResponse, Responder, Result as ActixResult};
use carve::config::Competition;
use carve::redis_manager::RedisManager;
use serde::{Deserialize, Serialize};

use crate::types;

#[derive(Deserialize)]
pub struct DeleteApiKeyRequest {
    pub api_key: String,
}

#[derive(Serialize)]
pub struct ApiKeyResponse {
    pub api_key: String,
}

#[derive(Serialize)]
pub struct ApiKeysListResponse {
    pub api_keys: Vec<String>,
}

#[get("/start_competition")]
async fn start_competition(
    redis: web::Data<RedisManager>,
    competition: web::Data<Competition>,
) -> ActixResult<impl Responder> {
    let competition_name = &competition.name;
    let duration = competition.duration;
    match redis.start_competition(competition_name, duration).await {
        Ok(_) => Ok(HttpResponse::Ok().body("Competition started")),
        Err(e) => {
            Ok(HttpResponse::InternalServerError()
                .body(format!("Failed to start competition: {}", e)))
        }
    }
}

#[get("/end_competition")]
async fn end_competition(
    redis: web::Data<RedisManager>,
    competition: web::Data<Competition>,
) -> ActixResult<impl Responder> {
    let competition_name = &competition.name;
    match redis.end_competition(competition_name).await {
        Ok(_) => Ok(HttpResponse::Ok().body("Competition stopped")),
        Err(e) => {
            Ok(HttpResponse::InternalServerError()
                .body(format!("Failed to stop competition: {}", e)))
        }
    }
}

#[get("/generate_join_code")]
pub async fn generate_join_code(
    query: web::Query<types::AdminGenerateCodeQuery>,
    competition: web::Data<Competition>,
    redis: web::Data<RedisManager>,
) -> ActixResult<impl Responder> {
    if query.team_name.is_empty() {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Admin must provide a team name"
        })));
    }
    // generate a join code for the team
    match redis
        .generate_team_join_code(&competition.name, &query.team_name)
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

/// Generate a new API key
#[post("/api_keys")]
pub async fn create_api_key(redis: web::Data<RedisManager>) -> ActixResult<impl Responder> {
    match redis.generate_api_key().await {
        Ok(api_key) => Ok(HttpResponse::Ok().json(ApiKeyResponse { api_key })),
        Err(_) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Failed to generate API key"
        }))),
    }
}

/// Get all API keys
#[get("/api_keys")]
pub async fn get_api_keys(redis: web::Data<RedisManager>) -> ActixResult<impl Responder> {
    match redis.get_api_keys().await {
        Ok(api_keys) => Ok(HttpResponse::Ok().json(ApiKeysListResponse { api_keys })),
        Err(_) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Failed to retrieve API keys"
        }))),
    }
}

/// Delete an API key
#[delete("/api_keys")]
pub async fn delete_api_key(
    redis: web::Data<RedisManager>,
    req: web::Json<DeleteApiKeyRequest>,
) -> ActixResult<impl Responder> {
    if req.api_key.is_empty() {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "API key cannot be empty"
        })));
    }

    match redis.remove_api_key(&req.api_key).await {
        Ok(_) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "message": "API key deleted successfully"
        }))),
        Err(_) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Failed to delete API key"
        }))),
    }
}
