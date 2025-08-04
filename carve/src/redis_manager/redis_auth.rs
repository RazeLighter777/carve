use super::*;

impl RedisManager {
    // Set a local user password using argon2i hashing
    pub async fn set_user_local_password(
        &self,
        competition_name: &str,
        username: &str,
        password: &str,
    ) -> Result<()> {
        use argon2::password_hash::{SaltString, rand_core::OsRng};
        use argon2::{Argon2, PasswordHasher};

        let password_hashes_key = self.competition_key(competition_name, "users:password_hashes");

        // Generate a salt and hash the password
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?
            .to_string();

        self.redis_hset(&password_hashes_key, username, password_hash)
            .await?;

        // Re-register the user with the new identity source LocalUserPassword
        let user_data_key = self.competition_key(competition_name, "user_data");
        if let Some(user_data_str) = self
            .redis_hget::<_, _, String>(&user_data_key, username)
            .await?
        {
            if let Some(mut user) = User::from_redis_format(&user_data_str) {
                if !user
                    .identity_sources
                    .contains(&IdentitySources::LocalUserPassword)
                {
                    user.identity_sources
                        .push(IdentitySources::LocalUserPassword);
                    self.redis_hset(&user_data_key, username, user.to_redis_format())
                        .await?;
                }
            }
        }

        Ok(())
    }

    // verify a local user password for a user. This is used for local authentication.
    // the method will return the User's object if the username/password combination is valid, false otherwise.
    pub async fn verify_user_local_password(
        &self,
        competition_name: &str,
        username: &str,
        password: &str,
    ) -> Result<Option<User>> {
        let password_hashes_key = self.competition_key(competition_name, "users:password_hashes");

        if let Some(hashed_password) = self
            .redis_hget::<_, _, String>(&password_hashes_key, username)
            .await?
        {
            let hashed_password = argon2::password_hash::PasswordHash::new(&hashed_password)
                .map_err(|e| anyhow::anyhow!("Failed to parse hashed password: {}", e))?;
            let hasher = argon2::Argon2::default();

            if hasher
                .verify_password(password.as_bytes(), &hashed_password)
                .is_ok()
            {
                let user_data_key = self.competition_key(competition_name, "user_data");
                if let Some(user_data_str) = self
                    .redis_hget::<_, _, String>(&user_data_key, username)
                    .await?
                {
                    return Ok(User::from_redis_format(&user_data_str));
                }
            }
        }
        Ok(None)
    }
}
