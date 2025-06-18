use actix_session::{Session, SessionExt};
use actix_web::guard::GuardContext;


pub fn validate_session(
    ctx : &GuardContext,
) -> bool {
    let session = ctx.get_session();
    if let Some(username) = session.get::<String>("username").unwrap_or(None) {
        if !username.is_empty() {
            return true;
        }
    }  
    false
}