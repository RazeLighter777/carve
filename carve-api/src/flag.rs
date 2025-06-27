use actix_session::Session;
use actix_web::guard::GuardContext;
use actix_web::{get, web, HttpResponse, Responder, Result as ActixResult};
use crate::types;
use carve::config::Competition;
use carve::redis_manager::RedisManager;

pub fn validate_bearer_token_is_secret_key_env_var(ctx : &GuardContext) -> bool {
    if let Some(auth_header) = ctx.head().headers().get("Authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Bearer ") {
                let token = &auth_str[7..];
                if let Ok(secret_key) = std::env::var("SECRET_KEY") {
                    return token == secret_key;
                }
            }
        }
    }
    false
}


// this method is only for internal use by other services, not exposed to users
// it takes a GenerateFlagQuery and returns a GenerateFlagResponse
// it calls the redis manager to generate a flag for the given competition, flag check name and team.
#[get("/generate_flag")]
pub async fn generate_flag(
    redis: web::Data<RedisManager>,
    competition: web::Data<Competition>,
    query: web::Query<types::GenerateFlagQuery>,
) -> ActixResult<impl Responder> {
    let competition_name = &competition.name;
    let flag_check_name = &query.flag_check_name;
    let team_name = &query.team_name;
    match redis.generate_new_flag(competition_name, team_name, flag_check_name) {
        Ok(flag) => {
            let response = types::GenerateFlagResponse { flag };
            Ok(HttpResponse::Ok().json(response))
        },
        Err(e) => {
            Ok(HttpResponse::InternalServerError().body(format!("Failed to generate flag: {}", e)))
        }
    }
}

