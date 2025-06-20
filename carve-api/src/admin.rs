// Admin-related functionality for the Carve API


use actix_web::{get, web, HttpResponse, Responder, Result as ActixResult};
use carve::config::Competition;
use carve::redis_manager::RedisManager;





#[get("/start_competition")]
async fn start_competition(
    redis: web::Data<RedisManager>,
    competition : web::Data<Competition>,
) -> ActixResult<impl Responder> {
    let competition_name = &competition.name;
    let duration = competition.duration;
    match redis.start_competition(competition_name, duration) {
        Ok(_) => {
            return Ok(HttpResponse::Ok().body("Competition started"));
        },
        Err(e) => {
            return Ok(HttpResponse::InternalServerError().body(format!("Failed to start competition: {}", e)));
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
            return Ok(HttpResponse::Ok().body("Competition stopped"));
        },
        Err(e) => {
            return Ok(HttpResponse::InternalServerError().body(format!("Failed to stop competition: {}", e)));
        }
    }
}