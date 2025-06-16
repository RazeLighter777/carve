use chrono::Utc;
use log::{error, info};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

use crate::check::perform_check;
use carve::config::Competition;
use carve::redis_manager::RedisManager;

pub struct Scheduler {
    competition: Competition,
    redis_manager: Arc<RedisManager>,
}

impl Scheduler {
    pub fn new(competition: Competition, redis_manager: Arc<RedisManager>) -> Self {
        Self {
            competition,
            redis_manager,
        }
    }
    
    pub async fn run(self) {
        for check in &self.competition.checks {
            let check = check.clone();
            let competition_name = self.competition.name.clone();
            let boxes = self.competition.boxes.clone();
            let teams = self.competition.teams.clone();
            let redis_manager = Arc::clone(&self.redis_manager);
            
            tokio::spawn(async move {
                loop {
                    let now = Utc::now().timestamp();
                    let interval = check.interval as i64;
                    
                    // Calculate time to next check
                    let time_to_next_check = interval - (now % interval);
                    sleep(Duration::from_secs(time_to_next_check as u64)).await;
                    
                    // Timestamp for this check round (aligned to interval)
                    let check_timestamp = (Utc::now().timestamp() / interval) * interval;
                    
                    // Process the check for all applicable boxes and teams
                    for team in &teams {
                        for box_config in &boxes {
                            // Create an empty HashMap to use as a fallback
                            let empty_selector: HashMap<String, String> = HashMap::new();
                            
                            // Check if this box matches the label selector
                            let label_selector = check.label_selector.as_ref()
                                .or(check.label_selector_alt.as_ref())
                                .unwrap_or(&empty_selector);
                            
                            // If label selector is empty, apply to all boxes
                            // Otherwise, check if box labels match
                            let should_check = label_selector.is_empty() || 
                                match label_selector.get("") {
                                    Some(label) => box_config.labels == *label,
                                    None => false
                                };
                            
                            if should_check {
                                // Replace {{.TEAM}} placeholder in hostname with actual team name
                                let hostname = format!("{}.{}.{}.local", 
                                    box_config.name, 
                                    team.name, 
                                    competition_name);
                                // launch dig with cmd to resolve the hostname to an IP address with the vtep's DNS server
                                let ip = match std::process::Command::new("dig")
                                    .arg(&hostname)
                                    .arg("@vtep")
                                    .arg("+short")
                                    .output() {
                                        Ok(output) if output.status.success() => {
                                            String::from_utf8_lossy(&output.stdout)
                                                .trim()
                                                .to_string()
                                        }
                                        _ => {
                                            error!("Failed to resolve hostname: {}", hostname);
                                            continue;
                                        }
                                    };
                                // check if we got a valid IP address
                                let ip = match ip.parse::<std::net::IpAddr>() {
                                    Ok(ip) => ip,
                                    Err(_) => {
                                        println!("Box {}.{}.{}.local has no dns entry (yet), skipping",
                                            box_config.name,
                                            team.name,
                                            competition_name);
                                        continue;
                                    }
                                };

                                info!(
                                    "Running check {} for team {} on box {} ({})",
                                    check.name, team.name, box_config.name, ip
                                );
                                
                                // Perform the check
                                match perform_check(&ip.to_string(), &check.spec).await {
                                    Ok(message) => {
                                        info!("Check successful: {}", message);
                                        let timestamp_ms = check_timestamp * 1000;
                                        
                                        if let Err(e) = redis_manager.record_sucessful_check_result(
                                            &competition_name,
                                            &check.name,
                                            timestamp_ms,
                                            &team.name,
                                            &box_config.name,
                                            &message,
                                        ) {
                                            error!("Failed to record check result: {}", e);
                                        }
                                    }
                                    Err(e) => {
                                        error!(
                                            "Check failed for {} on {}: {}",
                                            team.name, box_config.name, e
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            });
        }
    }
}
