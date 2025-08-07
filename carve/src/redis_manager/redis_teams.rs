use std::collections::HashMap;

use super::*;

impl RedisManager {
    pub async fn generate_team_join_code(
        &self,
        competition_name: &str,
        team_name: &str,
    ) -> Result<u64> {
        let mut conn = self.get_connection().await?;
        let join_code: u64 = rand::random::<u64>() % 1_000_000_000;
        let key = self.competition_key(competition_name, "team_join_codes");

        self.redis_hset(&key, join_code, team_name).await?;

        redis::cmd("HEXPIRE")
            .arg(&key)
            .arg(86400)
            .arg("FIELDS")
            .arg(1)
            .arg(join_code)
            .query_async::<()>(&mut conn)
            .await
            .context("Failed to set expiration for team join code")?;
        Ok(join_code)
    }

    pub async fn check_team_join_code(
        &self,
        competition_name: &str,
        join_code: u64,
    ) -> Result<Option<String>> {
        let key = self.competition_key(competition_name, "team_join_codes");
        self.redis_hget(&key, join_code).await
    }

    // Get all users for a team
    pub async fn get_team_users(
        &self,
        competition_name: &str,
        team_name: &str,
    ) -> Result<Vec<User>> {
        let mut conn = self.get_connection().await?;

        let team_users_key = format!("{}:{}:users", competition_name, team_name);
        let users: Vec<String> = redis::cmd("SMEMBERS")
            .arg(&team_users_key)
            .query_async(&mut conn)
            .await
            .context("Failed to get team users")?;

        let mut result = Vec::new();
        for user_name in users {
            // use redis.get_user_data to get the user data
            if let Ok(Some(userdata)) = self.get_user(competition_name, &user_name).await {
                result.push(userdata);
            }
        }

        Ok(result)
    }
    pub async fn get_team_with_least_members(
        &self,
        competition_name: &str,
    ) -> Result<Option<String>> {
        match self.get_all_users(competition_name).await {
            Ok(users) => {
                let mut team_members: HashMap<String, usize> = HashMap::new();
                for user in users {
                    if let Some(team) = &user.team_name {
                        *team_members.entry(team.clone()).or_insert(0) += 1;
                    }
                }

                // Find the team with the least members
                let min_team = team_members
                    .iter()
                    .min_by_key(|&(_, count)| count)
                    .map(|(team, _)| team.clone());

                Ok(min_team)
            }
            Err(e) => Err(e),
        }
    }
}
