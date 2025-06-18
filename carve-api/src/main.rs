use actix_session::{storage::CookieSessionStore, Session, SessionMiddleware};
use actix_web::cookie::Key;
use actix_web::{
    get, middleware::Logger, web, App, HttpResponse, HttpServer, Responder, Result as ActixResult,
};
use carve::redis_manager::User;
use carve::{
    config::{AppConfig, Competition},
    redis_manager::RedisManager,
};
use chrono::{DateTime, Utc};
use env_logger::Env;
use reqwest;
use oauth2::{basic::*, PkceCodeVerifier, TokenResponse};
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge, RedirectUrl,
    Scope, TokenUrl,
};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::process::Stdio;
use tokio::process::Command;
// API Response structures
#[derive(Serialize)]
struct UserResponse {
    name: String,
    email: String,
    #[serde(rename = "teamId")]
    team_id: u64,
}

#[derive(Serialize)]
struct TeamMember {
    name: String,
}

#[derive(Serialize)]
struct TeamResponse {
    id: u64,
    name: String,
    members: Vec<TeamMember>,
}

type OauthClient = oauth2::Client<
    oauth2::StandardErrorResponse<BasicErrorResponseType>,
    oauth2::StandardTokenResponse<oauth2::EmptyExtraTokenFields, BasicTokenType>,
    oauth2::StandardTokenIntrospectionResponse<oauth2::EmptyExtraTokenFields, BasicTokenType>,
    oauth2::StandardRevocableToken,
    oauth2::StandardErrorResponse<oauth2::RevocationErrorResponseType>,
    oauth2::EndpointSet,
    oauth2::EndpointNotSet,
    oauth2::EndpointNotSet,
    oauth2::EndpointNotSet,
    oauth2::EndpointSet,
>;

#[derive(Serialize)]
struct CompetitionResponse {
    status: String,
    name: String,
    #[serde(rename = "startDate")]
    start_date: DateTime<Utc>,
    #[serde(rename = "endDate")]
    end_date: DateTime<Utc>,
}

#[derive(Serialize)]
struct ScoreEvent {
    id: u64,
    #[serde(rename = "teamId")]
    team_id: u64,
    #[serde(rename = "scoringCheck")]
    scoring_check: String,
    timestamp: DateTime<Utc>,
    message: String,
}

#[derive(Serialize)]
struct LeaderboardEntry {
    #[serde(rename = "teamId")]
    team_id: u64,
    #[serde(rename = "teamName")]
    team_name: String,
    score: i64,
    rank: u64,
}

#[derive(Serialize)]
struct LeaderboardResponse {
    teams: Vec<LeaderboardEntry>,
}

#[derive(Serialize)]
struct BoxInfo {
    name: String,
}

#[derive(Serialize)]
struct BoxDetailResponse {
    name: String,
    #[serde(rename = "ipAddress")]
    ip_address: String,
    status: String,
}

#[derive(Serialize)]
struct BoxCredentialsResponse {
    name: String,
    username: String,
    password: String,
}

#[derive(Serialize)]
struct CheckResponse {
    name: String,
    points: u32,
}

#[derive(Serialize)]
struct TeamListEntry {
    id: u64,
    name: String,
}

#[derive(Serialize)]
struct TeamsResponse {
    teams: Vec<TeamListEntry>,
}

// Query parameters
#[derive(Deserialize)]
struct UserQuery {
    username: String,
}

#[derive(Deserialize)]
struct TeamQuery {
    id: u64,
}

#[derive(Deserialize)]
struct BoxesQuery {
    #[serde(rename = "teamId")]
    team_id: u64,
}

#[derive(Deserialize)]
struct BoxQuery {
    name: String,
}

#[derive(Deserialize)]
struct ScoreQuery {
    #[serde(rename = "teamId")]
    team_id: Option<u64>,
    #[serde(rename = "scoringCheck")]
    scoring_check: Option<String>,
    #[serde(rename = "startDate")]
    start_date: Option<DateTime<Utc>>,
    #[serde(rename = "endDate")]
    end_date: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
struct OauthCallBackQuery {
    code: String,
    state: String,
}
// Helper function to resolve IP address using dig
async fn resolve_box_ip(box_name: &str, vtep_host: &str) -> Option<String> {
    let output = Command::new("dig")
        .arg(box_name)
        .arg(&format!("@{}", vtep_host))
        .arg("+short")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await;

    match output {
        Ok(output) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let ip = stdout.trim();

                // Validate IPv4 address format
                if !ip.is_empty() && is_valid_ipv4(ip) {
                    Some(ip.to_string())
                } else {
                    None
                }
            } else {
                None
            }
        }
        Err(_) => None,
    }
}

// Helper function to validate IPv4 address format
fn is_valid_ipv4(ip: &str) -> bool {
    let parts: Vec<&str> = ip.split('.').collect();
    if parts.len() != 4 {
        return false;
    }

    for part in parts {
        if let Ok(_) = part.parse::<u8>() {
            // Valid if it's a number between 0-255
            if part.len() > 1 && part.starts_with('0') {
                return false; // No leading zeros allowed
            }
        } else {
            return false;
        }
    }

    true
}

// API Handlers
#[get("/api/v1/user")]
async fn get_user(
    query: web::Query<UserQuery>,
    competition: web::Data<Competition>,
    redis: web::Data<RedisManager>,
) -> ActixResult<impl Responder> {
    let username = query.username.clone();
    // Get user from Redis
    match redis.get_user(&competition.name, &username) {
        Ok(Some(user)) => {
            let team_id = if let Some(ref team_name) = user.team_name {
                competition
                    .teams
                    .iter()
                    .position(|t| t.name == *team_name)
                    .map(|idx| idx as u64 + 1)
                    .unwrap_or(0)
            } else {
                0
            };

            let response = UserResponse {
                name: user.username,
                email: user.email,
                team_id,
            };
            Ok(HttpResponse::Ok().json(response))
        }
        Ok(None) => Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "User not found"
        }))),
        Err(_) => Ok(HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Failed to retrieve user"
        }))),
    }
}

#[get("/api/v1/team")]
async fn get_team(
    query: web::Query<TeamQuery>,
    competition: web::Data<Competition>,
    redis: web::Data<RedisManager>,
) -> ActixResult<impl Responder> {
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
            .map(|user| TeamMember {
                name: user.username,
            })
            .collect(),
        Err(_) => {
            // Return empty members list if Redis query fails
            Vec::new()
        }
    };

    let team_response = TeamResponse {
        id: team_id as u64,
        name: team.name.clone(),
        members,
    };

    Ok(HttpResponse::Ok().json(team_response))
}

#[get("/api/v1/teams")]
async fn get_teams(competition: web::Data<Competition>) -> ActixResult<impl Responder> {
    let teams: Vec<TeamListEntry> = competition
        .teams
        .iter()
        .enumerate()
        .map(|(idx, team)| TeamListEntry {
            id: idx as u64 + 1,
            name: team.name.clone(),
        })
        .collect();

    let response = TeamsResponse { teams };
    Ok(HttpResponse::Ok().json(response))
}

#[get("/api/v1/competition")]
async fn get_competition(competition: web::Data<Competition>) -> ActixResult<impl Responder> {
    let response = CompetitionResponse {
        status: "active".to_string(),
        name: competition.name.clone(),
        start_date: Utc::now(),                             // Mock start date
        end_date: Utc::now() + chrono::Duration::hours(24), // Mock end date
    };

    Ok(HttpResponse::Ok().json(response))
}

#[get("/api/v1/score")]
async fn get_score(
    query: web::Query<ScoreQuery>,
    competition: web::Data<Competition>,
    redis: web::Data<RedisManager>,
) -> ActixResult<impl Responder> {
    let start_time = query
        .start_date
        .unwrap_or_else(|| Utc::now() - chrono::Duration::days(1))
        .timestamp_millis();
    let end_time = query
        .end_date
        .unwrap_or_else(|| Utc::now())
        .timestamp_millis();

    let mut scores = Vec::new();
    let mut score_id = 1u64;

    // If team_id is specified, filter by team
    let teams_to_check: Vec<_> = if let Some(team_id) = query.team_id {
        if team_id as usize > competition.teams.len() || team_id == 0 {
            return Ok(HttpResponse::NotFound().json(serde_json::json!({
                "error": "Team not found"
            })));
        }
        vec![(
            team_id,
            competition.teams[team_id as usize - 1].name.clone(),
        )]
    } else {
        competition
            .teams
            .iter()
            .enumerate()
            .map(|(i, team)| (i as u64 + 1, team.name.clone()))
            .collect()
    };

    // If scoring_check is specified, filter by check
    let checks_to_check: Vec<_> = if let Some(ref check_name) = query.scoring_check {
        if let Some(check) = competition.checks.iter().find(|c| c.name == *check_name) {
            vec![check.clone()]
        } else {
            return Ok(HttpResponse::NotFound().json(serde_json::json!({
                "error": "Scoring check not found"
            })));
        }
    } else {
        competition.checks.clone()
    };

    // Get score events from Redis
    for (team_id, team_name) in teams_to_check {
        for check in &checks_to_check {
            match redis.get_team_score_check_events(
                &competition.name,
                &team_name,
                &check.name,
                start_time,
                end_time,
            ) {
                Ok(events) => {
                    for (unix_timestamp, _message) in events {
                        // Parse unix_timestamp from String to i64
                        let ts = unix_timestamp.parse::<i64>().unwrap_or(0);
                        scores.push(ScoreEvent {
                            id: score_id,
                            team_id,
                            scoring_check: check.name.clone(),
                            timestamp: DateTime::<Utc>::from_timestamp_millis(ts)
                                .unwrap_or_else(|| Utc::now()),
                            message: _message,
                        });
                        score_id += 1;
                    }
                }
                Err(_) => {
                    // Continue even if Redis query fails for this team/check combination
                }
            }
        }
    }

    Ok(HttpResponse::Ok().json(scores))
}

#[get("/api/v1/score/leaderboard")]
async fn get_leaderboard(
    competition: web::Data<Competition>,
    redis: web::Data<RedisManager>,
) -> ActixResult<impl Responder> {
    let mut leaderboard_entries = Vec::new();

    for (team_idx, team) in competition.teams.iter().enumerate() {
        let mut total_score = 0i64;

        // Calculate total score for this team across all checks
        for check in &competition.checks {
            match redis.get_team_score_by_check(
                &competition.name,
                &team.name,
                &check.name,
                check.points as i64,
            ) {
                Ok(score) => total_score += score,
                Err(_) => {
                    // Continue even if Redis query fails for this check
                }
            }
        }

        leaderboard_entries.push(LeaderboardEntry {
            team_id: team_idx as u64 + 1,
            team_name: team.name.clone(),
            score: total_score,
            rank: 0, // Will be set after sorting
        });
    }

    // Sort by score (descending) and assign ranks
    leaderboard_entries.sort_by(|a, b| b.score.cmp(&a.score));
    for (idx, entry) in leaderboard_entries.iter_mut().enumerate() {
        entry.rank = idx as u64 + 1;
    }

    let response = LeaderboardResponse {
        teams: leaderboard_entries,
    };

    Ok(HttpResponse::Ok().json(response))
}

#[get("/api/v1/boxes")]
async fn get_boxes(
    query: web::Query<BoxesQuery>,
    competition: web::Data<Competition>,
) -> ActixResult<impl Responder> {
    let team_id = query.team_id as usize;
    if team_id == 0 || team_id > competition.teams.len() {
        return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "Team not found"
        })));
    }

    let team_name = &competition.teams[team_id - 1].name;

    // Generate box names based on team name and available boxes
    let boxes: Vec<BoxInfo> = competition
        .boxes
        .iter()
        .map(|box_config| BoxInfo {
            name: format!(
                "{}.{}.{}.local",
                box_config.name,
                team_name.to_lowercase(),
                competition.name.to_lowercase()
            ),
        })
        .collect();

    Ok(HttpResponse::Ok().json(boxes))
}

#[get("/api/v1/box")]
async fn get_box(
    query: web::Query<BoxQuery>,
    competition: web::Data<Competition>,
) -> ActixResult<impl Responder> {
    // Parse box name to extract box type and team
    let parts: Vec<&str> = query.name.split('.').collect();
    if parts.len() < 3 {
        return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "Box not found"
        })));
    }

    let box_type = parts[0];
    let team_name = parts[1];

    // Verify the box type exists in configuration
    let box_exists = competition.boxes.iter().any(|b| b.name == box_type);

    // Verify the team exists
    let team_exists = competition
        .teams
        .iter()
        .any(|t| t.name.to_lowercase() == team_name);

    if !box_exists || !team_exists {
        return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "Box not found"
        })));
    }

    // Resolve IP address using dig if vtep_host is configured
    let ip_address = if let Some(ref vtep_host) = competition.vtep_host {
        resolve_box_ip(&query.name, vtep_host)
            .await
            .unwrap_or_else(|| "0.0.0.0".to_string()) // Fallback if resolution fails
    } else {
        "192.168.1.100".to_string() // Fallback if no vtep_host configured
    };

    let response = BoxDetailResponse {
        name: query.name.clone(),
        ip_address,
        status: "active".to_string(),
    };

    Ok(HttpResponse::Ok().json(response))
}

#[get("/api/v1/box/defaultCreds")]
async fn get_box_default_creds(
    query: web::Query<BoxQuery>,
    competition: web::Data<Competition>,
    redis: web::Data<RedisManager>,
) -> ActixResult<impl Responder> {
    // Parse box name to extract box type and team
    let parts: Vec<&str> = query.name.split('.').collect();
    if parts.len() < 3 {
        return Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "Creds not set"
        })));
    }

    let box_type = parts[0];
    let team_name = parts[1];

    // Try to get credentials from Redis
    match redis.read_box_credentials(&competition.name, team_name, box_type) {
        Ok(Some((username, password))) => {
            let response = BoxCredentialsResponse {
                name: query.name.clone(),
                username,
                password,
            };
            Ok(HttpResponse::Ok().json(response))
        }
        Ok(None) => Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "Creds not set"
        }))),
        Err(_) => Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "Creds not set"
        }))),
    }
}

#[get("/api/v1/checks")]
async fn get_checks(competition: web::Data<Competition>) -> ActixResult<impl Responder> {
    let checks: Vec<CheckResponse> = competition
        .checks
        .iter()
        .map(|check| CheckResponse {
            name: check.name.clone(),
            points: check.points,
        })
        .collect();

    Ok(HttpResponse::Ok().json(checks))
}

#[get("/api/v1/oauth2/get_oauth2_redirect_url")]
async fn get_oauth2_redirect_url(
    session: Session,
    client: web::Data<OauthClient>,
    redis: web::Data<RedisManager>,
) -> ActixResult<impl Responder> {
    // Generate CSRF token
    let csrf_token = CsrfToken::new_random();
    session.insert("csrf_token", csrf_token.secret())?;

    // Generate PKCE code challenge
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    // Build the authorization URL
    let (authorize_url, _csrf_state) = client
        .authorize_url(
            || csrf_token
        )
        .add_scope(Scope::new("openid".to_string()))
        .add_scope(Scope::new("profile".to_string()))
        .add_scope(Scope::new("email".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();
    // store verifier in session
    session.insert("pkce_verifier", pkce_verifier.secret())?;
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "redirectUrl": authorize_url.to_string(),
    })))
}

// callback endpoint for OAuth2
#[get("/api/v1/oauth2/callback")]
async fn oauth2_callback(
    query : web::Query<OauthCallBackQuery>,
    session: Session,
    client: web::Data<OauthClient>,
    redis: web::Data<RedisManager>,
    competition : web::Data<Competition>,
) -> ActixResult<impl Responder> {
    // get code and state from query parameters
    let code = query.code.clone();
    let state = query.state.clone();
    //get pkce_verifier from session
    let pkce_verifier: String = match session.get("pkce_verifier") {
        Ok(Some(verifier)) => verifier,
        _ => {
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "error": "PKCE verifier not found in session"
            })));
        }
    };
    //verify state matches csrf_token
    let csrf_token: String = match session.get("csrf_token") {
        Ok(Some(token)) => token,
        _ => {
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "error": "CSRF token not found in session"
            })));
        }
    };
    println!("State: {}, CSRF Token: {}", state, csrf_token);
    if state != csrf_token {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Invalid CSRF token"
        })));
    }
    //verify pkce_verifier
    let pkce_verifier = PkceCodeVerifier::new(pkce_verifier);
    let token_request = client.exchange_code(AuthorizationCode::new(code));

    match token_request.set_pkce_verifier(pkce_verifier).request_async(&oauth2::reqwest::ClientBuilder::new().redirect(reqwest::redirect::Policy::none()).use_native_tls().build().expect("Should build")).await {
        Ok(token) => {
            // Extract user information from token
            let oidc_userinfo_url = std::env::var("OAUTH2_USERINFO_URL")
                .expect("OAUTH2_USERINFO_URL not set");
            let userinfo_reqwest = reqwest::ClientBuilder::new()
                .use_native_tls()
                .build()
                .expect("Failed to build userinfo request");
            let userinfo_response = userinfo_reqwest
                .get(&oidc_userinfo_url)
                .bearer_auth(token.access_token().secret())
                .send()
                .await;
            // parse the userinfo response to json, then iterate through the groups field to find the team name
            match userinfo_response {
                Ok(response) => {
                    match response.json::<serde_json::Value>().await {
                        Ok(user_info) => {
                            let username = user_info["preferred_username"]
                                .as_str()
                                .unwrap_or("unknown")
                                .to_string();
                            let email = user_info["email"]
                                .as_str()
                                .unwrap_or("unknown")
                                .to_string();
                            
                            // get list of teams and find the team name in the groups field
                            let mut team_name: Option<String> = None;
                            let mut is_admin = false;
                            if let Some(groups) = user_info["groups"].as_array() {
                                for group in groups {
                                    if let Some(group_name) = group.as_str() {
                                        // Check if the group name matches any team name
                                        if competition.teams.iter().any(|t| t.name == group_name) {
                                            team_name = Some(group_name.to_string());
                                            break;
                                        }
                                        // Check if the group name matches the admin group
                                        if let Some(admin_group) = &competition.admin_group {
                                            if group_name == admin_group {
                                                is_admin = true;
                                            }
                                        }
                                    }
                                }
                            }

                            
                            // return an error response if no team is found
                            if team_name.is_none() {
                                return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                                    "error": "No team found for user"
                                })));
                            }
                            // call register_user in redis_manager
                            let team_name = team_name.unwrap();
                            match redis.register_user(
                                &competition.name,
                                &User {
                                    username: username.clone(),
                                    email: email.clone(),
                                    team_name: Some(team_name.clone()),
                                },
                                &team_name,
                            ) {
                                Ok(_) => {
                                    println!("User {} registered successfully", username);
                                }
                                Err(e) => {
                                    println!("Error registering user: {:?}", e);
                                    return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                                        "error": "Failed to register user"
                                    })));
                                }
                            }
                            // create session with user info
                            session.insert("username", username)?;
                            session.insert("email", email)?;
                            session.insert("team_name", team_name)?;
                            session.insert("is_admin", is_admin)?;

                        }
                        Err(e) => {
                            println!("Error parsing user info: {:?}", e);
                        }
                    }
                }
                Err(e) => {
                    println!("Error fetching user info: {:?}", e);
                }
            }
            
            session.insert("token", token)?;

            return Ok(HttpResponse::Ok().json(serde_json::json!({
                "message": "Authorization successful"
            })));
        },
        Err(e) => {
            println!("Error {:?}", e.source());
            return Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to exchange authorization code for token"
            })));
        }
    };
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    let competition_name =
        std::env::var("COMPETITION_NAME").unwrap_or_else(|_| "default".to_string());
    let config = AppConfig::new().expect("Failed to load configuration");
    let competition = config
        .competitions
        .iter()
        .find(|c| c.name == competition_name)
        .expect("Competition not found in configuration");
    let competition_clone = competition.clone();

    //read the SECRET_KEY from environment variable
    let secret_key = std::env::var("SECRET_KEY").expect("SECRET_KEY not set");
    let secret_key = Key::from(secret_key.as_bytes());
    println!("Starting server for competition: {}", competition.name);

    // get client_id and client_secret from environment variables
    let client_id = std::env::var("OAUTH2_CLIENT_ID").expect("OAUTH2_CLIENT_ID not set");
    let client_secret =
        std::env::var("OAUTH2_CLIENT_SECRET").expect("OAUTH2_CLIENT_SECRET not set");
    let auth_url = std::env::var("OAUTH2_AUTH_URL").expect("OAUTH2_AUTH_URL not set");
    let token_url = std::env::var("OAUTH2_TOKEN_URL").expect("OAUTH2_TOKEN_URL not set");
    let redirect_url = std::env::var("OAUTH2_REDIRECT_URL").expect("OAUTH2_REDIRECT_URL not set");

    let client: OauthClient = BasicClient::new(ClientId::new(client_id))
        .set_client_secret(ClientSecret::new(client_secret))
        .set_auth_uri(AuthUrl::new(auth_url).expect("Invalid auth URL"))
        .set_token_uri(TokenUrl::new(token_url).expect("Invalid token URL"))
        .set_redirect_uri(RedirectUrl::new(redirect_url).expect("Invalid redirect URL"));

    let redis_manager = RedisManager::new(&competition.redis).expect("Failed to connect to Redis");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(competition_clone.clone()))
            .app_data(web::Data::new(redis_manager.clone()))
            .app_data(web::Data::new(client.clone()))
            .wrap(Logger::default())
            .wrap(SessionMiddleware::new(
                CookieSessionStore::default(),
                secret_key.clone(),
            ))
            .service(get_user)
            .service(get_team)
            .service(get_teams)
            .service(get_competition)
            .service(get_score)
            .service(get_leaderboard)
            .service(get_boxes)
            .service(get_box)
            .service(get_box_default_creds)
            .service(get_checks)
            .service(get_oauth2_redirect_url)
            .service(oauth2_callback)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
