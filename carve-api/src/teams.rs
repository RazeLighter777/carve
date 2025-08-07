// Teams-related API handlers

use crate::types;
use actix_session::Session;
use actix_web::{get, post, web, HttpResponse, Responder, Result as ActixResult};
use carve::config::Competition;
use carve::redis_manager::RedisManager;

// Helper function to check if user is admin
async fn is_user_admin(redis: &RedisManager, competition_name: &str, username: &str) -> bool {
    match redis.get_user(competition_name, username).await {
        Ok(Some(user)) => user.is_admin,
        _ => false,
    }
}

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
    let teams = futures::future::join_all(competition.teams.iter().enumerate().map(
        async |(idx, team)| {
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
        },
    ))
    .await;

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
                match redis
                    .get_box_console_code(&competition.name, &team_name)
                    .await
                {
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
            if let Ok(Some(state)) = redis
                .get_check_current_state(&competition.name, &team_name, &check.name)
                .await
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
            if let Ok(Some(state)) = redis
                .get_check_current_state(&competition.name, &team_name, &flag_check.name)
                .await
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

// Support ticket routes

#[get("/team/support_tickets")]
pub async fn get_team_support_tickets(
    competition: web::Data<Competition>,
    redis: web::Data<RedisManager>,
    session: Session,
) -> ActixResult<impl Responder> {
    if let Some(team_name) = session.get::<String>("team_name")? {
        // Check if user is admin
        if let Some(username) = session.get::<String>("username")? {
            let is_admin = is_user_admin(&redis, &competition.name, &username).await;
            
            if is_admin {
                // Admin can see all tickets across all teams
                match redis.get_all_support_tickets(&competition.name).await {
                    Ok(all_tickets) => {
                        let ticket_responses: Vec<types::SupportTicketResponse> = all_tickets
                            .into_iter()
                            .map(|(_, ticket_id, ticket)| {
                                // Note: we're using the ticket's team_name from the data, not the session team_name
                                types::SupportTicketResponse {
                                    ticket_id,
                                    ticket,
                                }
                            })
                            .collect();
                        
                        Ok(HttpResponse::Ok().json(types::SupportTicketsResponse {
                            tickets: ticket_responses,
                        }))
                    }
                    Err(_) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": "Failed to retrieve support tickets"
                    }))),
                }
            } else {
                // Regular team member can only see their team's tickets
                match redis.get_team_support_tickets(&competition.name, &team_name).await {
                    Ok(tickets) => {
                        let ticket_responses: Vec<types::SupportTicketResponse> = tickets
                            .into_iter()
                            .map(|(ticket_id, ticket)| types::SupportTicketResponse {
                                ticket_id,
                                ticket,
                            })
                            .collect();
                        
                        Ok(HttpResponse::Ok().json(types::SupportTicketsResponse {
                            tickets: ticket_responses,
                        }))
                    }
                    Err(_) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": "Failed to retrieve support tickets"
                    }))),
                }
            }
        } else {
            Ok(HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "Username not found in session"
            })))
        }
    } else {
        Ok(HttpResponse::Unauthorized().json(serde_json::json!({
            "error": "Team name not found in session"
        })))
    }
}

#[get("/team/support_ticket")]
pub async fn get_support_ticket(
    query: web::Query<types::SupportTicketQuery>,
    competition: web::Data<Competition>,
    redis: web::Data<RedisManager>,
    session: Session,
) -> ActixResult<impl Responder> {
    if let Some(team_name) = session.get::<String>("team_name")? {
        if let Some(username) = session.get::<String>("username")? {
            let is_admin = is_user_admin(&redis, &competition.name, &username).await;
            
            // If admin, try to find the ticket across all teams
            if is_admin {
                // Get all tickets and find the one with matching ID
                match redis.get_all_support_tickets(&competition.name).await {
                    Ok(all_tickets) => {
                        if let Some((_, _, ticket)) = all_tickets.into_iter().find(|(_, ticket_id, _)| *ticket_id == query.ticket_id) {
                            Ok(HttpResponse::Ok().json(types::SupportTicketResponse {
                                ticket_id: query.ticket_id,
                                ticket,
                            }))
                        } else {
                            Ok(HttpResponse::NotFound().json(serde_json::json!({
                                "error": "Support ticket not found"
                            })))
                        }
                    }
                    Err(_) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": "Failed to retrieve support tickets"
                    }))),
                }
            } else {
                // Regular team member can only see their team's tickets
                match redis.get_support_ticket(&competition.name, &team_name, query.ticket_id).await {
                    Ok(Some(ticket)) => {
                        Ok(HttpResponse::Ok().json(types::SupportTicketResponse {
                            ticket_id: query.ticket_id,
                            ticket,
                        }))
                    }
                    Ok(_) => Ok(HttpResponse::NotFound().json(serde_json::json!({
                        "error": "Support ticket not found"
                    }))),
                    Err(_) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": "Failed to retrieve support ticket"
                    }))),
                }
            }
        } else {
            Ok(HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "Username not found in session"
            })))
        }
    } else {
        Ok(HttpResponse::Unauthorized().json(serde_json::json!({
            "error": "Team name not found in session"
        })))
    }
}

#[post("/team/support_ticket")]
pub async fn create_support_ticket(
    request: web::Json<types::CreateSupportTicketRequest>,
    competition: web::Data<Competition>,
    redis: web::Data<RedisManager>,
    session: Session,
) -> ActixResult<impl Responder> {
    if let Some(team_name) = session.get::<String>("team_name")? {
        if request.message.trim().is_empty() {
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Message cannot be empty"
            })));
        }

        match redis.create_support_ticket(&competition.name, &team_name, &request.message, &request.subject).await {
            Ok(ticket_id) => {
                Ok(HttpResponse::Created().json(types::CreateSupportTicketResponse {
                    ticket_id,
                    message: "Support ticket created successfully".to_string(),
                }))
            }
            Err(_) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to create support ticket"
            }))),
        }
    } else {
        Ok(HttpResponse::Unauthorized().json(serde_json::json!({
            "error": "Team name not found in session"
        })))
    }
}

#[post("/team/support_ticket/message")]
pub async fn add_support_ticket_message(
    query: web::Query<types::SupportTicketQuery>,
    request: web::Json<types::AddSupportTicketMessageRequest>,
    competition: web::Data<Competition>,
    redis: web::Data<RedisManager>,
    session: Session,
) -> ActixResult<impl Responder> {
    if let Some(username) = session.get::<String>("username")? {
        if request.message.trim().is_empty() {
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Message cannot be empty"
            })));
        }

        // Check if user is admin
        let is_admin = is_user_admin(&redis, &competition.name, &username).await;
        
        if is_admin {
            // Admin can reply to any team's ticket - get team from ticket or use provided team
            let team_name = if let Some(team) = session.get::<String>("team_name")? {
                team
            } else {
                return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
                    "error": "No team context found"
                })));
            };

            match redis.add_support_ticket_message(
                &competition.name,
                &team_name,
                query.ticket_id,
                "admin",
                &request.message,
            ).await {
                Ok(()) => {
                    Ok(HttpResponse::Ok().json(serde_json::json!({
                        "message": "Admin message added to support ticket successfully"
                    })))
                }
                Err(e) if e.to_string().contains("not found") => {
                    Ok(HttpResponse::NotFound().json(serde_json::json!({
                        "error": "Support ticket not found"
                    })))
                }
                Err(_) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "Failed to add message to support ticket"
                }))),
            }
        } else {
            // Non-admin users can only reply to their team's tickets
            if let Some(team_name) = session.get::<String>("team_name")? {
                match redis.add_support_ticket_message(
                    &competition.name,
                    &team_name,
                    query.ticket_id,
                    "team",
                    &request.message,
                ).await {
                    Ok(()) => {
                        Ok(HttpResponse::Ok().json(serde_json::json!({
                            "message": "Message added to support ticket successfully"
                        })))
                    }
                    Err(e) if e.to_string().contains("not found") => {
                        Ok(HttpResponse::NotFound().json(serde_json::json!({
                            "error": "Support ticket not found"
                        })))
                    }
                    Err(_) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": "Failed to add message to support ticket"
                    }))),
                }
            } else {
                Ok(HttpResponse::Unauthorized().json(serde_json::json!({
                    "error": "Team name not found in session"
                })))
            }
        }
    } else {
        Ok(HttpResponse::Unauthorized().json(serde_json::json!({
            "error": "Username not found in session"
        })))
    }
}

#[post("/team/support_ticket/status")]
pub async fn update_support_ticket_status(
    query: web::Query<types::SupportTicketQuery>,
    request: web::Json<types::UpdateSupportTicketStatusRequest>,
    competition: web::Data<Competition>,
    redis: web::Data<RedisManager>,
    session: Session,
) -> ActixResult<impl Responder> {
    if let Some(username) = session.get::<String>("username")? {
        // Only admins can update ticket status
        let is_admin = is_user_admin(&redis, &competition.name, &username).await;
        
        if !is_admin {
            return Ok(HttpResponse::Forbidden().json(serde_json::json!({
                "error": "Only administrators can update ticket status"
            })));
        }

        if let Some(team_name) = session.get::<String>("team_name")? {
            match redis.update_support_ticket_status(
                &competition.name,
                &team_name,
                query.ticket_id,
                &request.status,
            ).await {
                Ok(()) => {
                    Ok(HttpResponse::Ok().json(serde_json::json!({
                        "message": format!("Support ticket status updated to: {}", request.status)
                    })))
                }
                Err(e) if e.to_string().contains("not found") => {
                    Ok(HttpResponse::NotFound().json(serde_json::json!({
                        "error": "Support ticket not found"
                    })))
                }
                Err(_) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "Failed to update support ticket status"
                }))),
            }
        } else {
            Ok(HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "Team context not found in session"
            })))
        }
    } else {
        Ok(HttpResponse::Unauthorized().json(serde_json::json!({
            "error": "Username not found in session"
        })))
    }
}
