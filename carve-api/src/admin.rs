// Admin-related functionality for the Carve API


use actix_web::{get, web, HttpResponse, Responder, Result as ActixResult};
use carve::config::Competition;
use carve::redis_manager::RedisManager;

use crate::types;





#[get("/start_competition")]
async fn start_competition(
    redis: web::Data<RedisManager>,
    competition : web::Data<Competition>,
) -> ActixResult<impl Responder> {
    let competition_name = &competition.name;
    let duration = competition.duration;
    match redis.start_competition(competition_name, duration) {
        Ok(_) => {
            Ok(HttpResponse::Ok().body("Competition started"))
        },
        Err(e) => {
            Ok(HttpResponse::InternalServerError().body(format!("Failed to start competition: {}", e)))
        }
    }
}

#[get("/end_competition")]
async fn end_competition(
    redis: web::Data<RedisManager>,
    competition: web::Data<Competition>,
) -> ActixResult<impl Responder> {
    let competition_name = &competition.name;
    match redis.end_competition(competition_name) {
        Ok(_) => {
            Ok(HttpResponse::Ok().body("Competition stopped"))
        },
        Err(e) => {
            Ok(HttpResponse::InternalServerError().body(format!("Failed to stop competition: {}", e)))
        }
    }
}

#[get("/generate_join_code")]
pub async fn generate_join_code(
    query : web::Query<types::AdminGenerateCodeQuery>,
    competition: web::Data<Competition>,
    redis: web::Data<RedisManager>,
) -> ActixResult<impl Responder> {
    if query.team_name.is_empty() {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Admin must provide a team name"
        })));
    }
    // generate a join code for the team
    match redis.generate_team_join_code(&competition.name, &query.team_name) {
        Ok(join_code) => Ok(HttpResponse::Ok().json(serde_json::json!({
            "code": join_code,
        }))),
        Err(_) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Failed to generate join code"
        }))),
    }
}
