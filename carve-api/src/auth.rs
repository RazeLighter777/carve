use std::error::Error;

use crate::types;
use actix_session::{Session, SessionExt};
use actix_web::cookie::Cookie;
use actix_web::guard::GuardContext;
use actix_web::{get, web, HttpResponse, Responder, Result as ActixResult};
use carve::config::Competition;
use carve::redis_manager::{RedisManager, User};
use oauth2::{
    AuthorizationCode, CsrfToken, PkceCodeChallenge, PkceCodeVerifier, Scope, TokenResponse,
};

pub fn validate_admin_session(ctx: &GuardContext) -> bool {
    let session = ctx.get_session();
    if let Some(_) = session.get::<String>("username").unwrap_or(None) {
        if let Ok(Some(is_admin)) = session.get::<bool>("is_admin") {
            if is_admin {
                return true;
            }
        }
    }
    println!("Session is invalid or username not found or user is not admin.");
    false
}

pub fn validate_session(ctx: &GuardContext) -> bool {
    let session = ctx.get_session();
    if let Some(username) = session.get::<String>("username").unwrap_or(None) {
        if !username.is_empty() {
            return true;
        }
    }
    println!("Session is invalid or username not found.");
    false
}

#[get("/get_oauth2_redirect_url")]
pub async fn get_oauth2_redirect_url(
    session: Session,
    client: web::Data<types::OauthClient>,
    competition : web::Data<Competition>,
) -> ActixResult<impl Responder> {
    // check if OIDC is a valid identity source for the competition
    if !competition.identity_sources.contains(&carve::redis_manager::IdentitySources::OIDC) {
        return Ok(HttpResponse::Found()
            .append_header(("Location", "/login?error=internal_error"))
            .finish());
    }
    // Generate CSRF token
    let csrf_token = CsrfToken::new_random();
    session.insert("csrf_token", csrf_token.secret())?;

    // Generate PKCE code challenge
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    // Build the authorization URL
    let (authorize_url, _csrf_state) = client
        .authorize_url(|| csrf_token)
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
#[get("/callback")]
async fn oauth2_callback(
    query: web::Query<types::OauthCallBackQuery>,
    session: Session,
    client: web::Data<types::OauthClient>,
    redis: web::Data<RedisManager>,
    competition: web::Data<Competition>,
) -> ActixResult<impl Responder> {
    // check if the OIDC identity source is configured for the competition
    if !competition.identity_sources.contains(&carve::redis_manager::IdentitySources::OIDC) {
        return Ok(HttpResponse::Found()
            .append_header(("Location", "/login?error=internal_error"))
            .finish());
    }
    // get code and state from query parameters
    let code = query.code.clone();
    let state = query.state.clone();
    //get pkce_verifier from session
    let pkce_verifier: String = match session.get("pkce_verifier") {
        Ok(Some(verifier)) => verifier,
        _ => {
            return Ok(HttpResponse::Found()
                .append_header(("Location", "/login?error=pkce"))
                .finish());
        }
    };
    //verify state matches csrf_token
    let csrf_token: String = match session.get("csrf_token") {
        Ok(Some(token)) => token,
        _ => {
            return Ok(HttpResponse::Found()
                .append_header(("Location", "/login?error=csrf"))
                .finish());
        }
    };
    println!("State: {}, CSRF Token: {}", state, csrf_token);
    if state != csrf_token {
        return Ok(HttpResponse::Found()
            .append_header(("Location", "/login?error=csrf"))
            .finish());
    }
    //verify pkce_verifier
    let pkce_verifier = PkceCodeVerifier::new(pkce_verifier);
    let token_request = client.exchange_code(AuthorizationCode::new(code));

    match token_request
        .set_pkce_verifier(pkce_verifier)
        .request_async(
            &oauth2::reqwest::ClientBuilder::new()
                .redirect(reqwest::redirect::Policy::none())
                .use_native_tls()
                .build()
                .expect("Should build"),
        )
        .await
    {
        Ok(token) => {
            // Extract user information from token
            let oidc_userinfo_url =
                std::env::var("OAUTH2_USERINFO_URL").expect("OAUTH2_USERINFO_URL not set");
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
                            let email =
                                user_info["email"].as_str().unwrap_or("unknown").to_string();
                            let mut team_name: Option<String> = redis
                                .get_user(&competition.name, &username)
                                .unwrap_or(None)
                                .and_then(|u| u.team_name);
                            let mut is_admin = false;
                            if let Some(groups) = user_info["groups"].as_array() {
                                for group in groups {
                                    if let Some(group_name) = group.as_str() {
                                        if competition.registration_type
                                            == carve::config::RegistrationType::OidcOnly
                                        {
                                            // get list of teams and find the team name in the groups field. If the team_name is not None, do not set the team_name again
                                            println!(
                                                "Group: {}, admin group: {}",
                                                group_name,
                                                &competition
                                                    .admin_group
                                                    .as_deref()
                                                    .unwrap_or("None")
                                            );
                                            // Check if the group name matches any team name
                                            if competition
                                                .teams
                                                .iter()
                                                .any(|t| t.name == group_name)
                                                && team_name.is_none()
                                            {
                                                team_name = Some(group_name.to_string());
                                                break;
                                            }
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

                            let user = User {
                                username: username.clone(),
                                email: email.clone(),
                                team_name: team_name.clone(),
                                is_admin,
                                identity_sources: vec![carve::redis_manager::IdentitySources::OIDC],
                            };
                            // call register_user in redis_manager
                            let register_result =
                                redis.register_user(&competition.name, &user, team_name.as_deref());
                            match register_result {
                                Ok(_) => {
                                    println!("User {} registered successfully", username);
                                }
                                Err(e) => {
                                    println!("Error registering user: {:?}", e);
                                    return Ok(HttpResponse::Found()
                                        .append_header(("Location", "/login?error=register"))
                                        .finish());
                                }
                            }
                            // create session with user info
                            session.insert("username", username)?;
                            session.insert("email", email)?;
                            session.insert("team_name", team_name)?;
                            session.insert("is_admin", is_admin)?;

                            // create cookies
                            let cookie =
                                Cookie::build("userinfo", serde_json::to_string(&user).unwrap())
                                    .path("/")
                                    .http_only(false)
                                    .finish();
                            Ok(HttpResponse::Found()
                                .append_header(("Location", "/"))
                                .cookie(cookie)
                                .finish())
                        }
                        Err(e) => {
                            println!("Error parsing user info: {:?}", e);
                            Ok(HttpResponse::Found()
                                .append_header(("Location", "/login?error=userinfo"))
                                .finish())
                        }
                    }
                }
                Err(e) => {
                    println!("Error fetching user info: {:?}", e);
                    Ok(HttpResponse::Found()
                        .append_header(("Location", "/login?error=userinfo"))
                        .finish())
                }
            }
        }
        Err(e) => {
            println!("Error {:?}", e.source());
            Ok(HttpResponse::Found()
                .append_header(("Location", "/login?error=token"))
                .finish())
        }
    }
}

#[get("/logout")]
pub async fn logout(session: Session) -> impl Responder {
    session.purge();
    // Optionally, you can also clear the userinfo cookie
    let mut userinfo_cookie = Cookie::build("userinfo", "")
        .path("/")
        .http_only(false)
        .finish();
    userinfo_cookie.make_removal();
    HttpResponse::Ok()
        .cookie(userinfo_cookie)
        .body("Logged out successfully")
}

//Traditional password login endpoint
#[get("/login")]
pub async fn login(
    session: Session,
    query: web::Query<types::LoginUserQuery>,
    redis: web::Data<RedisManager>,
    competition: web::Data<Competition>,
) -> ActixResult<impl Responder> {
    // check if LocalUserPassword is a valid identity source for the competition
    if !competition.identity_sources.contains(
        &carve::redis_manager::IdentitySources::LocalUserPassword,
    ) {
        return Ok(HttpResponse::Found()
            .append_header(("Location", "/login?error=internal_error"))
            .finish());
    }
    // Check if the user is already logged in
    if let Some(username) = session.get::<String>("username").unwrap_or(None) {
        if !username.is_empty() {
            // User is already logged in, redirect to home
            return Ok(HttpResponse::Found()
                .append_header(("Location", "/"))
                .finish());
        }
    }

    // verify the username/password against redis
    match redis.verify_user_local_password(&competition.name, &query.username, &query.password) {
        Ok(Some(user)) => {
            // create session with user info
            session.insert("username", user.username.clone())?;
            session.insert("email", user.email.clone())?;
            session.insert("team_name", user.team_name.clone())?;
            session.insert("is_admin", user.is_admin)?;

            // create cookies
            let cookie = Cookie::build("userinfo", serde_json::to_string(&user).unwrap())
                .path("/")
                .http_only(false)
                .finish();
            return Ok(HttpResponse::Found()
                .append_header(("Location", "/"))
                .cookie(cookie)
                .finish());
        }
        Err(e) => {
            println!("Error verifying user: {:?}", e);
            return Ok(HttpResponse::Found()
                .append_header(("Location", "/login?error=internal_error"))
                .finish());
        }
        Ok(None) => {
            // User not found or password incorrect
            return Ok(HttpResponse::Found()
                .append_header(("Location", "/login?error=invalid_credentials"))
                .finish());
        }
    }
}

//Traditional password registration endpoint
#[get("/register")]
pub async fn register(
    session: Session,
    query: web::Query<types::RegistrationQuery>,
    redis: web::Data<RedisManager>,
    competition: web::Data<Competition>,
) -> ActixResult<impl Responder> {
    // check if LocalUserPassword is a valid identity source for the competition
    if !competition.identity_sources.contains(
        &carve::redis_manager::IdentitySources::LocalUserPassword,
    ) {
        return Ok(HttpResponse::Found()
            .append_header(("Location", "/register?error=internal_error"))
            .finish());
    }
    // Check if the user is already logged in
    if let Some(username) = session.get::<String>("username").unwrap_or(None) {
        if !username.is_empty() {
            // User is already logged in, redirect to home
            return Ok(HttpResponse::Found()
                .append_header(("Location", "/"))
                .finish());
        }
    }
    let mut team_name = None;
    // Check if the team join code is valid, if provided
    if let Some(join_code) = query.team_join_code {
        if let Ok(Some(team)) = redis
            .check_team_join_code(&competition.name, join_code)
            .map_err(|e| {
                println!("Error checking team join code: {:?}", e);
                HttpResponse::Found()
                    .append_header(("Location", "/register?error=internal_error"))
                    .finish()
            })
        {
            team_name = Some(team);
        }
    }
    // Check if the username already exists
    if let Ok(Some(_)) = redis.get_user(&competition.name, &query.username) {
        return Ok(HttpResponse::Found()
            .append_header(("Location", "/register?error=username_exists"))
            .finish());
    }
    // Create a new user
    let user = User {
        username: query.username.clone(),
        email: query.email.clone(),
        team_name: team_name.clone(),
        is_admin: false,
        identity_sources: vec![carve::redis_manager::IdentitySources::LocalUserPassword],
    };
    // Register the user in Redis
    match redis.register_user(&competition.name, &user, user.team_name.as_deref()) {
        Ok(_) => {
            match redis.set_user_local_password(&competition.name, &query.username, &query.password)
            {
                Ok(_) => {
                    // redirect to login page with success message
                    return Ok(HttpResponse::Found()
                        .append_header(("Location", "/login?success=registered"))
                        .finish());
                }
                Err(e) => {
                    println!("Error setting user password: {:?}", e);
                    return Ok(HttpResponse::Found()
                        .append_header(("Location", "/register?error=password_requirements_not_met"))
                        .finish());
                }
            }
        }
        Err(e) => {
            println!("Error registering user: {:?}", e);
            return Ok(HttpResponse::Found()
                .append_header(("Location", "/register?error=internal_error"))
                .finish());
        }
    }
}

// returns a list of identity sources configured for the competition
#[get("/identity_sources")]
pub async fn identity_sources(
    competition: web::Data<Competition>,
) -> ActixResult<impl Responder> {
    let sources = &competition.identity_sources;
    Ok(HttpResponse::Ok().json(types::IdentitySourcesResponse {
        sources: sources.clone(),
    }))
}