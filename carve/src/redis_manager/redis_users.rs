use super::*;

impl RedisManager {
    pub async fn get_all_users(&self, competition_name: &str) -> Result<Vec<User>> {
        let mut conn = self.get_connection().await?;
        let key = format!("{}:users", competition_name);
        let user_data_key = format!("{}:user_data", competition_name);
        // Get all usernames in the competition
        let usernames: Vec<String> = redis::cmd("SMEMBERS")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .context("Failed to get usernames")?;
        let mut users = Vec::new();
        for username in usernames {
            // Get user data for each username
            let user_data_str: String = redis::cmd("HGET")
                .arg(&user_data_key)
                .arg(&username)
                .query_async(&mut conn)
                .await
                .context("Failed to get user data")?;
            if let Some(user) = User::from_redis_format(&user_data_str) {
                users.push(user);
            } else {
                return Err(anyhow::anyhow!(
                    "Failed to deserialize user data for username: {}",
                    username
                ));
            }
        }
        Ok(users)
    }

    // Register a user to a team. Creates/inserts a new key in competition_name:users and competition_name:team_name:users
    // competition_name:users -> set of usernames, emails.
    // takes a User struct and team_name
    // if user already exists, register them to the new team, check iteratively which team they are registered to, and remove them from the old team.
    pub async fn register_user(
        &self,
        competition_name: &str,
        user: &User,
        team_name: Option<&str>,
    ) -> Result<()> {
        util::validate_user_fields(user)
            .map_err(|e| anyhow::anyhow!("Invalid user fields: {}", e))?;
        let users_key = self.competition_key(competition_name, "users");
        let users_data_key = self.competition_key(competition_name, "user_data");

        let mut updated_user = if let Some(existing_user_data) = self
            .redis_hget::<_, _, String>(&users_data_key, &user.username)
            .await?
        {
            let mut existing_user = User::from_redis_format(&existing_user_data)
                .context("Failed to deserialize existing user data")?;
            for new_source in &user.identity_sources {
                if !existing_user.identity_sources.contains(new_source) {
                    existing_user.identity_sources.push(new_source.clone());
                }
            }
            existing_user.email = user.email.clone();
            existing_user.team_name = user.team_name.clone();
            existing_user
        } else {
            self.redis_sadd(&users_key, &user.username).await?;
            user.clone()
        };

        if let Some(team_name) = team_name {
            self.move_user_to_team(competition_name, &user.username, team_name)
                .await?;
            updated_user.team_name = Some(team_name.to_string());
        }

        self.redis_hset(
            &users_data_key,
            &user.username,
            updated_user.to_redis_format(),
        )
        .await?;
        Ok(())
    }

    // Get a specific user by username and find their team
    pub async fn get_user(&self, competition_name: &str, username: &str) -> Result<Option<User>> {
        let users_key = self.competition_key(competition_name, "users");
        let user_data_key = self.competition_key(competition_name, "user_data");

        let mut conn = self.get_connection().await?;
        let user_exists: bool = redis::cmd("SISMEMBER")
            .arg(&users_key)
            .arg(username)
            .query_async(&mut conn)
            .await
            .context("Failed to check if user exists")?;

        if user_exists {
            if let Some(data) = self
                .redis_hget::<_, _, String>(&user_data_key, username)
                .await?
            {
                return Ok(User::from_redis_format(&data));
            }
        }
        Ok(None)
    }

    async fn move_user_to_team(
        &self,
        competition_name: &str,
        username: &str,
        new_team: &str,
    ) -> Result<()> {
        let mut conn = self.get_connection().await?;
        let pattern = format!("{}:*:users", competition_name);
        let team_keys: Vec<String> = redis::cmd("KEYS")
            .arg(&pattern)
            .query_async(&mut conn)
            .await?;
        for team_key in team_keys {
            let _: () = redis::cmd("SREM")
                .arg(&team_key)
                .arg(username)
                .query_async(&mut conn)
                .await?;
        }
        let new_team_key = self.team_key(competition_name, new_team, "users");
        self.redis_sadd(&new_team_key, username).await?;
        Ok(())
    }
}
