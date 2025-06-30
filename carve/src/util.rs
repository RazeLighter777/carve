use crate::redis_manager::User;
use regex::Regex;

pub fn validate_user_fields(user: &User) -> Result<(), String> {
    // username must be at least 3 characters long, no more than 32 characters
    // may only contain _, -, and alphanumeric characters
    // may not start with a number
    if user.username.len() < 3 || user.username.len() > 32 {
        return Err("Username must be between 3 and 32 characters long".to_string());
    }
    if !user
        .username
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return Err("Username may only contain _, -, and alphanumeric characters".to_string());
    }
    if user.username.chars().next().unwrap().is_numeric() {
        return Err("Username may not start with a number".to_string());
    }
    // email must be a valid email address, matching regex
    let email_regex = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$")
        .map_err(|_| "Invalid email regex".to_string())?;
    if !email_regex.is_match(&user.email) {
        return Err("Email must be a valid email address".to_string());
    }
    Ok(())
}

pub fn validate_password(password: &str) -> Result<(), String> {
    // Password must be at least 8 characters long. that's it
    if password.len() < 8 {
        return Err("Password must be at least 8 characters long".to_string());
    }
    Ok(())
}
