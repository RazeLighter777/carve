// Teams-related API handlers

use crate::types;
use actix_session::Session;
use actix_web::{get, web, HttpResponse, Responder, Result as ActixResult};
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
    let members = match redis.get_team_users(&competition.name, &team.name).await {
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
pub async fn get_teams(
    competition: web::Data<Competition>,
    redis: web::Data<RedisManager>,
) -> ActixResult<impl Responder> {
    // ...existing code from main.rs...
    let teams= futures::future::join_all(competition
        .teams
        .iter()
        .enumerate()
        .map(async |(idx, team)| {
            let members = redis
                .get_team_users(&competition.name, &team.name)
                .await
                .unwrap_or_default()
                .into_iter()
                .map(|user| types::TeamMember {
                    name: user.username,
                })
                .collect();
            types::TeamListEntry {
                members,
                id: idx as u64 + 1,
                name: team.name.clone(),
            }
        })).await;

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
        // Check if competition is active
        match redis.get_competition_state(&competition.name).await {
            Ok(state) if state.status == carve::redis_manager::CompetitionStatus::Active => {
                // Get the console code from Redis
                match redis.get_box_console_code(&competition.name, &team_name).await {
                    Ok(code) => Ok(HttpResponse::Ok().json(serde_json::json!({
                        "code": code
                    }))),

                    Err(_) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": "Failed to retrieve console code"
                    }))),
                }
            }
            Ok(_) => Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Competition is not active"
            }))),

            Err(_) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to retrieve competition state"
            }))),
        }
    } else {
        Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Team name not found in session"
        })))
    }
}

#[get("/team/check_status")]
pub async fn get_team_check_status(
    query: web::Query<types::TeamCheckStatusQuery>,
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
            if let Ok(Some(state)) =
                redis.get_check_current_state(&competition.name, &team_name, &check.name).await
            {
                response.checks.push(types::CheckStatusResponse {
                    name: check.name.clone(),
                    passing: state.success,
                    failed_for: state.number_of_failures,
                    message: state.message.clone(),
                    success_fraction: state.success_fraction,
                    passing_boxes: state.passing_boxes,
                });
            }
        }
        for flag_check in competition.flag_checks.iter() {
            if let Ok(Some(state)) =
                redis.get_check_current_state(&competition.name, &team_name, &flag_check.name).await
            {
                response.flag_checks.push(types::FlagCheckStatusResponse {
                    name: flag_check.name.clone(),
                    passing: state.success,
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
