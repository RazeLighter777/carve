// Teams-related API handlers

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
pub async fn get_teams(competition: web::Data<Competition>) -> ActixResult<impl Responder> {
    // ...existing code from main.rs...
    let teams: Vec<types::TeamListEntry> = competition
        .teams
        .iter()
        .enumerate()
        .map(|(idx, team)| types::TeamListEntry {
            id: idx as u64 + 1,
            name: team.name.clone(),
        })
        .collect();

    let response = types::TeamsResponse { teams };
    Ok(HttpResponse::Ok().json(response))
}
