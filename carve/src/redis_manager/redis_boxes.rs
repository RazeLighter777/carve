use crate::config::ToastNotification;
use crate::config::ToastSeverity;

use super::*;

impl RedisManager {
    // Generates a box console code for a team. This is a unique code that can be used to access the team's boxes,
    // and is passed to novnc proxy in the url path.
    // This is a 32 character alphanumeric code.
    // if the code already exists, it will return the existing code.
    pub async fn get_box_console_code(
        &self,
        competition_name: &str,
        team_name: &str,
    ) -> Result<String> {
        let key = self.competition_key(competition_name, "box_console_codes");

        if let Some(console_code) = self.redis_hget::<_, _, String>(&key, team_name).await? {
            return Ok(console_code);
        }

        let console_code = Self::generate_alphanumeric_string(32);
        self.redis_hset(&key, team_name, &console_code).await?;
        Ok(console_code)
    }

    // wait for events for qemu boxes.
    // this blocking call takes an iterator of events, and waits one event to happen.
    pub async fn wait_for_qemu_event(
        &self,
        competition_name: &str,
        team_name: &str,
        box_name: &str,
        events: impl Iterator<Item = QemuCommands> + Clone,
    ) -> Result<QemuCommands> {
        // the key name
        let key = format!("{}:{}:{}:events", competition_name, team_name, box_name);

        // Subscribe to the key for events
        let (mut sink, mut stream) = self
            .client
            .get_async_pubsub()
            .await
            .context("Failed to get Redis pubsub connection")?
            .split();
        sink.subscribe(&key)
            .await
            .context("Failed to subscribe to Redis channel")?;

        // Return next event that matches one of the commands
        loop {
            let msg = stream
                .next()
                .await
                .context("Failed to receive message from Redis")?;
            // check if the message is a valid QEMU command
            if let Ok(command) = serde_yaml::from_str::<QemuCommands>(
                &msg.get_payload::<String>()
                    .context("Failed to get payload from Redis message")?,
            ) {
                if events.clone().any(|e| e == command) {
                    return Ok(command);
                }
            }
        }
    }

    pub async fn send_qemu_event(
        &self,
        competition_name: &str,
        team_name: &str,
        box_name: &str,
        command: QemuCommands,
    ) -> Result<()> {
        let mut conn = self.get_connection().await?;

        // the key name
        let key = format!("{}:{}:{}:events", competition_name, team_name, box_name);

        // Publish the command as a YAML string
        let payload =
            serde_yaml::to_string(&command).context("Failed to serialize QEMU command")?;
        let _: () = redis::cmd("PUBLISH")
            .arg(&key)
            .arg(payload)
            .query_async(&mut conn)
            .await
            .context("Failed to publish QEMU command")?;
        // publish a warning toast notification to the team
        self.publish_toast(&ToastNotification {
            title: "Box Event".to_string(),
            message: format!("Box '{}' has received a '{:?}' command.", box_name, command),
            severity: ToastSeverity::Warning,
            user: None,
            team: Some(team_name.to_string()),
        }).await?;
        Ok(())
    }

    pub async fn create_cooldown(
        &self,
        competition_name: &str,
        team_name: &str,
        box_name: &str,
        cooldown_seconds: u64,
    ) -> Result<()> {
        let mut conn = self.get_connection().await?;

        // the key name
        let key = format!("{}:{}:{}:cooldown", competition_name, team_name, box_name);

        // Set the cooldown with an expiration time
        let _: () = redis::cmd("SET")
            .arg(&key)
            .arg("active")
            .arg("EX")
            .arg(cooldown_seconds)
            .query_async(&mut conn)
            .await
            .context("Failed to create cooldown")?;

        Ok(())
    }

    pub async fn is_cooldown_ready(
        &self,
        competition_name: &str,
        team_name: &str,
        box_name: &str,
    ) -> Option<i64> {
        // check if key is expiring, and if it is return time left with TTL
        let mut conn = match self.client.get_multiplexed_tokio_connection().await {
            Ok(conn) => conn,
            Err(_) => return None, // Return None if connection fails
        };
        // the key name
        let key = format!("{}:{}:{}:cooldown", competition_name, team_name, box_name);
        // Check if the cooldown key exists
        let ttl: i64 = redis::cmd("TTL")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .context("Failed to check cooldown TTL")
            .ok()?;
        // redis returns -2 if the key does not exist, -1 if it exists but has no expiration
        if ttl == -2 {
            return None; // Cooldown does not exist
        } else if ttl == -1 {
            return Some(0); // Cooldown exists but has no expiration
        }
        // If the key exists, return the remaining TTL
        Some(ttl) // Return the remaining TTL in seconds
    }

    pub async fn create_vxlan_fdb_entry(
        &self,
        competition_name: &str,
        mac_address: &str,
        ip_address: IpAddr,
        domain: &str,
    ) -> Result<()> {
        let mut conn = self.get_connection().await?;
        let key = format!("{}:vxlan_fdb:{}", competition_name, domain);

        self.redis_hset(&key, mac_address, ip_address.to_string())
            .await?;

        redis::cmd("HEXPIRE")
            .arg(&key)
            .arg(20)
            .arg("FIELDS")
            .arg(1)
            .arg(mac_address)
            .query_async::<()>(&mut conn)
            .await
            .context("Failed to set expiration for VXLAN FDB entry")?;
        Ok(())
    }

    pub async fn get_domain_fdb_entries(
        &self,
        competition_name: &str,
        domain: &str,
    ) -> Result<Vec<(String, String)>> {
        let mut conn = self.get_connection().await?;
        let key = format!("{}:vxlan_fdb:{}", competition_name, domain);

        let entries: Vec<String> = redis::cmd("HGETALL")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .context("Failed to get VXLAN FDB entries")?;

        Ok(entries
            .chunks(2)
            .map(|chunk| (chunk[0].to_string(), chunk[1].to_string()))
            .collect())
    }

    pub async fn record_box_ip(
        &self,
        competition_name: &str,
        team_name: &str,
        box_name: &str,
        ip_address: IpAddr,
    ) -> Result<()> {
        let key = self.box_key(competition_name, team_name, box_name, "ip_address");
        let mut conn = self.get_connection().await?;
        redis::cmd("SET")
            .arg(&key)
            .arg(ip_address.to_string())
            .query_async::<()>(&mut conn)
            .await
            .context("Failed to record box IP address")
    }

    // Helper method for box data operations
    async fn write_box_data(
        &self,
        competition_name: &str,
        team_name: &str,
        box_name: &str,
        suffix: &str,
        data: &str,
    ) -> Result<bool> {
        let mut conn = self.get_connection().await?;
        let key = self.box_key(competition_name, team_name, box_name, suffix);
        let res: Option<String> = redis::cmd("SET")
            .arg(&key)
            .arg(data)
            .arg("NX")
            .query_async(&mut conn)
            .await
            .with_context(|| format!("Failed to write box {}", suffix))?;
        Ok(res.is_some())
    }

    async fn read_box_data(
        &self,
        competition_name: &str,
        team_name: &str,
        box_name: &str,
        suffix: &str,
    ) -> Result<Option<String>> {
        let mut conn = self.get_connection().await?;
        let key = self.box_key(competition_name, team_name, box_name, suffix);
        redis::cmd("GET")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .with_context(|| format!("Failed to read box {}", suffix))
    }

    // Write SSH keypair for a box. Returns true if written, false if key exists.
    pub async fn write_ssh_keypair(
        &self,
        competition_name: &str,
        team_name: &str,
        box_name: &str,
        private_key: &str,
    ) -> Result<bool> {
        self.write_box_data(
            competition_name,
            team_name,
            box_name,
            "ssh_keypair",
            private_key,
        )
        .await
    }

    // Read SSH keypair for a box. Returns None if not found.
    pub async fn read_ssh_keypair(
        &self,
        competition_name: &str,
        team_name: &str,
        box_name: &str,
    ) -> Result<Option<String>> {
        self.read_box_data(competition_name, team_name, box_name, "ssh_keypair")
            .await
    }

    // Write username/password for a box. Returns true if written, false if key exists.
    pub async fn write_box_credentials(
        &self,
        competition_name: &str,
        team_name: &str,
        box_name: &str,
        username: &str,
        password: &str,
    ) -> Result<bool> {
        let value = format!("{}:{}", username, password);
        self.write_box_data(competition_name, team_name, box_name, "credentials", &value)
            .await
    }

    // Read username/password for a box. Returns None if not found.
    pub async fn read_box_credentials(
        &self,
        competition_name: &str,
        team_name: &str,
        box_name: &str,
    ) -> Result<Option<(String, String)>> {
        if let Some(val) = self
            .read_box_data(competition_name, team_name, box_name, "credentials")
            .await?
        {
            let mut parts = val.splitn(2, ':');
            if let (Some(username), Some(password)) = (parts.next(), parts.next()) {
                return Ok(Some((username.to_string(), password.to_string())));
            }
        }
        Ok(None)
    }

    pub async fn record_sucessful_check_result(
        &self,
        competition_name: &str,
        check_name: &str,
        timestamp: DateTime<chrono::Utc>,
        team_id: u64,
        occurances: u64,
    ) -> Result<String> {
        let key = format!("{}:{}:{}", competition_name, team_id, check_name);
        // Only record if competition is Active
        let state = self.get_competition_state(competition_name).await?;
        if state.status != CompetitionStatus::Active {
            // Do nothing, just return the key name
            return Ok(key);
        }
        let mut conn = self.get_connection().await?;
        let timestamp_seconds = timestamp.timestamp();
        for i in 0..occurances {
            let _: () = redis::cmd("ZADD")
                .arg(&key)
                .arg(timestamp_seconds)
                .arg(format!("{}:{}", timestamp_seconds, i))
                .query_async(&mut conn)
                .await
                .context("Failed to record successful check result")?;
        }
        Ok(key)
    }

    // Get detailed teams scores by check
    pub async fn get_team_score_by_check(
        &self,
        competition_name: &str,
        team_id: u64,
        check_name: &str,
        check_points: i64,
    ) -> Result<i64> {
        let mut conn = self.get_connection().await?;

        // the key name
        let key = format!("{}:{}:{}", competition_name, team_id, check_name);

        // Get the total score for this team in this competition
        let score: i64 = redis::cmd("ZCARD")
            .arg(&key)
            .query_async(&mut conn)
            .await
            .context("Failed to get team score")?;

        // multiply by the number of points per check
        let score = score * check_points;

        Ok(score)
    }

    pub async fn set_check_current_state(
        &self,
        competition_name: &str,
        team_name: &str,
        check_name_or_flag_check_name: &str,
        success: bool,
        number_of_failures: u64,
        messages: Vec<String>,
        success_fraction: (u64, u64), // fraction of successful checks over total checks
        passing_boxes: Vec<String>,
    ) -> Result<()> {
        let key = self.team_key(competition_name, team_name, "current_state");
        let state = CheckCurrentState {
            success,
            number_of_failures,
            message: messages,
            success_fraction,
            passing_boxes,
        };
        let status = Self::serialize_to_yaml(&state)?;
        self.redis_hset(&key, check_name_or_flag_check_name, status)
            .await
    }

    pub async fn get_check_current_state(
        &self,
        competition_name: &str,
        team_name: &str,
        check_name_or_flag_check_name: &str,
    ) -> Result<Option<CheckCurrentState>> {
        let key = self.team_key(competition_name, team_name, "current_state");

        if let Some(state_str) = self
            .redis_hget::<_, _, String>(&key, check_name_or_flag_check_name)
            .await?
        {
            match Self::deserialize_from_yaml(&state_str) {
                Ok(parsed) => return Ok(Some(parsed)),
                Err(e) => {
                    return Err(anyhow::anyhow!(
                        "Invalid state format (YAML): {}: {}",
                        state_str,
                        e
                    ));
                }
            }
        }

        Ok(Some(CheckCurrentState {
            success: false,
            number_of_failures: 0,
            message: Vec::from(["Unsolved".to_string()]),
            success_fraction: (0, 0),
            passing_boxes: Vec::new(),
        }))
    }

    /// Get the score for a team for a specific check up to a given timestamp (inclusive).
    /// Returns the total score (number of successful events * check points).
    pub async fn get_number_of_successful_checks_at_time(
        &self,
        competition_name: &str,
        team_id: u64,
        check_name: &str,
        timestamp: i64,
    ) -> Result<i64> {
        let mut conn = self.get_connection().await?;
        // the key name
        let key = format!("{}:{}:{}", competition_name, team_id, check_name);
        // Get the number of events for this team/check up to the timestamp
        let count: i64 = redis::cmd("ZCOUNT")
            .arg(&key)
            .arg("-inf")
            .arg(timestamp)
            .query_async(&mut conn)
            .await
            .context("Failed to get team score by check at time")?;
        // Try to get the check points from the check or flag_check (not available here, so just return count)
        Ok(count)
    }

    pub async fn get_number_of_successful_checks_at_times(
        &self,
        competition_name: &str,
        team_id: u64,
        check_name: &str,
        timestamps: impl IntoIterator<Item = i64> + Clone,
    ) -> Result<Vec<i64>> {
        let mut conn = self.get_connection().await?;
        // the key name
        let key = format!("{}:{}:{}", competition_name, team_id, check_name);
        let mut pipeline = redis::pipe();
        pipeline.atomic();
        for timestamp in timestamps.clone() {
            pipeline.cmd("ZCOUNT").arg(&key).arg("-inf").arg(timestamp);
        }
        pipeline.query_async(&mut conn).await.context("Failed to get team scores by check at times")
    }
}
