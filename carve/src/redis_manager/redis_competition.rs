use crate::config::{ToastNotification, ToastSeverity};

use super::*;

impl RedisManager {
    // get the global competition state atomically. If the state is not set, will insert a default state (Unstarted).
    pub async fn get_competition_state(&self, competition_name: &str) -> Result<CompetitionState> {
        let mut conn = self.get_connection().await?;

        // Key for competition state
        let key = format!("{}:state", competition_name);

        // Try to get the state
        let state: Option<String> = redis::cmd("HGET")
            .arg(&key)
            .arg("state")
            .query_async(&mut conn)
            .await
            .context("Failed to get competition state")?;
        match state {
            None => {
                let default_state = CompetitionState {
                    name: competition_name.to_string(),
                    status: CompetitionStatus::Unstarted,
                    start_time: None,
                    end_time: None,
                };
                // Insert default state into Redis
                let _: () = redis::cmd("HSET")
                    .arg(&key)
                    .arg("state")
                    .arg(
                        serde_yaml::to_string(&default_state)
                            .context("Failed to serialize default state")?,
                    )
                    .query_async(&mut conn)
                    .await
                    .context("Failed to set default competition state")?;
                Ok(default_state)
            }
            Some(state_str) => {
                match serde_yaml::from_str::<CompetitionState>(&state_str) {
                    Ok(mut state) => {
                        // If the state is Active and end_time is set and now >= end_time, set to Finished
                        if state.status == CompetitionStatus::Active {
                            if let Some(end_time) = state.end_time {
                                let now = chrono::Utc::now();
                                if now >= end_time {
                                    state.status = CompetitionStatus::Finished;
                                    state.end_time = Some(end_time); // keep the original end_time
                                    // Store the updated state in Redis
                                    let _: () = redis::cmd("HSET")
                                        .arg(&key)
                                        .arg("state")
                                        .arg(
                                            serde_yaml::to_string(&state)
                                                .context("Failed to serialize finished state")?,
                                        )
                                        .query_async(&mut conn)
                                        .await
                                        .context(
                                            "Failed to update competition state to finished",
                                        )?;
                                    // Optionally publish the finished event
                                    let _: () = redis::cmd("PUBLISH")
                                        .arg(format!("{}:events", competition_name))
                                        .arg(serde_yaml::to_string(&state).context(
                                            "Failed to serialize finished state for publish",
                                        )?)
                                        .query_async(&mut conn)
                                        .await
                                        .context("Failed to publish competition finished event")?;
                                }
                            }
                        }
                        Ok(state)
                    }
                    Err(e) => Err(anyhow::anyhow!(
                        "Failed to deserialize competition state: {}",
                        e
                    )),
                }
            }
        }
    }

    //starts the copmetition. Returns an error if the competition is already started or finished.
    pub async fn start_competition(
        &self,
        competition_name: &str,
        duration: Option<u64>,
    ) -> Result<()> {
        let mut conn = self.get_connection().await?;
        // use get_competition_state to check current state
        let current_state = self.get_competition_state(competition_name).await?;
        match current_state.status {
            CompetitionStatus::Active => Err(anyhow::anyhow!("Competition is already active")),
            CompetitionStatus::Unstarted => {
                // Set new state to active with current timestamp
                let start_time = chrono::Utc::now();
                let end_time = duration.map(|d| start_time + chrono::Duration::seconds(d as i64));

                let new_state = CompetitionState {
                    name: competition_name.to_string(),
                    status: CompetitionStatus::Active,
                    start_time: Some(start_time),
                    end_time,
                };

                // Store the new state in Redis
                let key = format!("{}:state", competition_name);
                let _: () = redis::cmd("HSET")
                    .arg(&key)
                    .arg("state")
                    .arg(
                        serde_yaml::to_string(&new_state)
                            .context("Failed to serialize competition state")?,
                    )
                    .query_async(&mut conn)
                    .await
                    .context("Failed to start competition")?;
                // publish the start event to a channel
                let _: () = redis::cmd("PUBLISH")
                    .arg(format!("{}:events", competition_name))
                    .arg(
                        serde_yaml::to_string(&new_state)
                            .context("Failed to serialize competition state for publish")?,
                    )
                    .query_async(&mut conn)
                    .await
                    .context("Failed to publish competition start event")?;
                // publish a toast notification
                self.publish_toast(&ToastNotification {
                    title: "Competition Started".to_string(),
                    message: format!("The competition '{}' has started.", competition_name),
                    severity: ToastSeverity::Info,
                    user: None,
                    team: None,
                }).await
                .context("Failed to publish competition start toast notification")?;
                Ok(())
            }
            CompetitionStatus::Finished => Err(anyhow::anyhow!("Competition has already finished")),
        }
    }

    // Ends the competition. Returns an error if the competition is not active.
    pub async fn end_competition(&self, competition_name: &str) -> Result<()> {
        let mut conn = self.get_connection().await?;
        // use get_competition_state to check current state
        let current_state = self.get_competition_state(competition_name).await?;
        match current_state.status {
            CompetitionStatus::Active => {
                // Set new state to finished with current timestamp
                let end_time = chrono::Utc::now();

                let new_state = CompetitionState {
                    name: competition_name.to_string(),
                    status: CompetitionStatus::Finished,
                    start_time: current_state.start_time,
                    end_time: Some(end_time),
                };

                // Store the new state in Redis
                let key = format!("{}:state", competition_name);
                let _: () = redis::cmd("HSET")
                    .arg(&key)
                    .arg("state")
                    .arg(
                        serde_yaml::to_string(&new_state)
                            .context("Failed to serialize competition state")?,
                    )
                    .query_async(&mut conn)
                    .await
                    .context("Failed to end competition")?;
                // publish the end event to a channel
                let _: () = redis::cmd("PUBLISH")
                    .arg(format!("{}:events", competition_name))
                    .arg(
                        serde_yaml::to_string(&new_state)
                            .context("Failed to serialize competition state for publish")?,
                    )
                    .query_async(&mut conn)
                    .await
                    .context("Failed to publish competition end event")?;
                self.publish_toast(&ToastNotification {
                    title: "Competition Ended".to_string(),
                    message: format!("The competition '{}' has ended.", competition_name),
                    severity: ToastSeverity::Info,
                    user: None,
                    team: None,
                }).await
                .context("Failed to publish competition end toast notification")?;
                Ok(())
            }
            CompetitionStatus::Unstarted => Err(anyhow::anyhow!("Competition has not started yet")),
            CompetitionStatus::Finished => Err(anyhow::anyhow!("Competition has already finished")),
        }
    }

    pub fn wait_for_competition_event(&self, competition_name: &str) -> Result<CompetitionState> {
        let mut conn = self
            .client
            .get_connection()
            .context("Failed to connect to Redis")?;

        // Subscribe to the competition events channel
        let mut pubsub = conn.as_pubsub();
        pubsub
            .subscribe(format!("{}:events", competition_name))
            .context("Failed to subscribe to competition events")?;

        // Wait for a message
        let msg = pubsub
            .get_message()
            .context("Failed to get message from Redis")?;
        println!("Received message: {:?}", msg);
        let payload: String = msg
            .get_payload()
            .context("Failed to get payload from message")?;
        println!("Received payload: {}", payload);
        // Deserialize the competition state
        let state: CompetitionState =
            serde_yaml::from_str(&payload).context("Failed to deserialize competition state")?;

        // Unsubscribe from the channel
        pubsub
            .unsubscribe(format!("{}:events", competition_name))
            .context("Failed to unsubscribe from competition events")?;

        Ok(state)
    }
}
