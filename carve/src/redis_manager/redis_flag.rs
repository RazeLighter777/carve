use crate::config::{ToastNotification, ToastSeverity};

use super::*;

impl RedisManager {
    pub async fn generate_new_flag(
        &self,
        competition_name: &str,
        team_name: &str,
        flag_check_name: &str,
    ) -> Result<String> {
        let key = format!(
            "{}:{}:{}:flags",
            competition_name, team_name, flag_check_name
        );
        let value = format!(
            "{}{{{}}}",
            competition_name,
            Self::generate_lowercase_string(8)
        );
        self.redis_sadd(&key, &value).await?;
        Ok(value)
    }

    pub async fn redeem_flag(
        &self,
        competition_name: &str,
        team_name: &str,
        team_id: u64,
        flag: &str,
        flag_check: &FlagCheck,
    ) -> Result<bool> {
        let mut conn = self.get_connection().await?;

        // Key for storing flags
        let key = format!(
            "{}:{}:{}:flags",
            competition_name, team_name, flag_check.name
        );
        // Check if the flag exists
        let exists: bool = redis::cmd("SISMEMBER")
            .arg(&key)
            .arg(flag)
            .query_async(&mut conn)
            .await
            .context("Failed to check if flag exists")?;

        // create score event for the flag redemption
        if exists {
            // Record the successful flag redemption
            let timestamp = chrono::Utc::now();
            let event_message = format!("Flag redeemed: {}", flag);
            self.record_sucessful_check_result(
                competition_name,
                &flag_check.name,
                timestamp,
                team_id,
                1, // 1 occurrence for this flag redemption
            )
            .await?;
            // set the current state of the flag check to true
            self.set_check_current_state(
                competition_name,
                team_name,
                &flag_check.name,
                true,
                0, // No failures on successful flag redemption
                vec![event_message],
                (1, 1),     // 1 success out of 1 check
                Vec::new(), // No passing boxes for flag checks
            )
            .await?;
        }

        if exists {
            // Remove the flag from the set
            let _: () = redis::cmd("SREM")
                .arg(&key)
                .arg(flag)
                .query_async(&mut conn)
                .await
                .context("Failed to remove flag from set")?;
            // Publish a toast notification for the flag redemption
            self.publish_toast(&ToastNotification {
                title: "Flag Redeemed".to_string(),
                message: format!("Team '{}' redeemed the flag '{}'.", team_name, flag),
                severity: ToastSeverity::Info,
                user: None,
                team: Some(team_name.to_string()),
                sound_effect: Some("flag_redeemed".to_string()), // Optional sound effect
            })
            .await
            .context("Failed to publish flag redemption toast notification")?;
            Ok(true) // Flag redeemed successfully
        } else {
            Ok(false) // Flag does not exist
        }
    }
}
