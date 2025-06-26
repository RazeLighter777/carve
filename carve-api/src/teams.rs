// Teams-related API handlers

use actix_session::Session;
use actix_web::{get, web, HttpResponse, Responder, Result as ActixResult};
use crate::types;
use carve::config::Competition;
use carve::redis_manager::RedisManager;

#[get("/team")]
pub async fn get_team(
    query: web::Query<types::TeamQuery>,
    competition: web::Data<Competition>,
    redis: web::Data<RedisManager>,
) -> ActixResult<impl Responder> {
    // ...existing code from main.rs...
    let team_id = query.id as usize;
    if team_id == 0 || team_id > competition.teams.len() {
        return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "Team not found"
        })));
    }

    let team = &competition.teams[team_id - 1];

    // Get actual team members from Redis
    let members = match redis.get_team_users(&competition.name, &team.name) {
        Ok(users) => users
            .into_iter()
            .map(|user| types::TeamMember {
                name: user.username,
            })
            .collect(),
        Err(_) => {
            // Return empty members list if Redis query fails
            Vec::new()
        }
    };

    let team_response = types::TeamResponse {
        id: team_id as u64,
        name: team.name.clone(),
        members,
    };

    Ok(HttpResponse::Ok().json(team_response))
}

#[get("/teams")]
pub async fn get_teams(competition: web::Data<Competition>, redis: web::Data<RedisManager>) -> ActixResult<impl Responder> {
    // ...existing code from main.rs...
    let teams: Vec<types::TeamListEntry> = competition
        .teams
        .iter()
        .enumerate()
        .map(|(idx, team)| types::TeamListEntry {
            members : redis.get_team_users(&competition.name, &team.name)
                .unwrap_or_default()
                .into_iter()
                .map(|user| types::TeamMember { name: user.username })
                .collect(),
            id: idx as u64 + 1,
            name: team.name.clone(),
        })
        .collect();

    let response = types::TeamsResponse { teams };
    Ok(HttpResponse::Ok().json(response))
}

// gets the team's console code.
// takes no parameters, but reads the session to get the team name.
// from the team name, it calls the Redis manager to get the console code with get_box_console_code
#[get("/team/console_code")]
pub async fn get_team_console_code(
    competition: web::Data<Competition>,
    redis: web::Data<RedisManager>,
    session: Session,
) -> ActixResult<impl Responder> {
    if let Some(team_name) = session.get::<String>("team_name")? {
        // Get the console code from Redis
        let console_code = match redis.get_box_console_code(&competition.name, &team_name) {
            Ok(code) => code,
            Err(_) => {
                return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "Failed to retrieve console code"
                })));
            }
        };

        Ok(HttpResponse::Ok().json(serde_json::json!({
            "code": console_code
        })))
    } else {
        Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Team name not found in session"
        })))
    }
}


#[get("/team/check_status")]
pub async fn get_team_check_status(
    query : web::Query<types::TeamCheckStatusQuery>,
    competition: web::Data<Competition>,
    redis: web::Data<RedisManager>,
) -> ActixResult<impl Responder> {
    let team_id = query.team_id;
    if let Some(team_name) = competition.get_team_name_from_id(team_id) {
        let mut response = types::TeamCheckStatusResponse {
            flag_checks: Vec::new(),
            checks: Vec::new(),
        };
        for check in competition.checks.iter() {
            if let Ok(Some((passing, failed_for, message))) = redis.get_check_current_state(&competition.name, &team_name, &check.name) {
                response.checks.push(types::CheckStatusResponse {
                    name: check.name.clone(),
                    passing,
                    failed_for,
                    message,
                });
            }
        }
        for flag_check in competition.flag_checks.iter() {
            if let Ok(Some((passing, _, _))) = redis.get_check_current_state(&competition.name, &team_name, &flag_check.name) {
                response.flag_checks.push(types::FlagCheckStatusResponse {
                    name: flag_check.name.clone(),
                    passing,
                });
            }
        }
        Ok(HttpResponse::Ok().json(response))
    } else {
        Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "Team not found"
        })))
    }
}