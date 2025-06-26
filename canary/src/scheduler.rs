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
        let competition = self.competition.clone();
        let redis_manager = self.redis_manager.clone();
        for check in competition.clone().checks {
            let check = check.clone();
            let competition = competition.clone();
            let redis_manager = redis_manager.clone();
            
            tokio::spawn(async move {
                let competition_name = competition.clone().name;
                let teams = competition.clone().teams;
                let boxes = competition.clone().boxes;

                loop {
                    let now = Utc::now().timestamp();
                    let interval = check.interval as i64;
                    
                    // Calculate time to next check
                    let time_to_next_check = interval - (now % interval);
                    let check_timestamp = now + time_to_next_check;
                    sleep(Duration::from_secs(time_to_next_check as u64)).await;
                    
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
                                
                                // Get current check state from Redis
                                let (_, mut prev_failures) = match redis_manager.get_check_current_state(
                                    &competition_name,
                                    &team.name,
                                    &check.name,
                                ) {
                                    Ok(Some((passing, failures, _))) => (passing, failures),
                                    _ => (true, 0), // Default: passing, 0 failures
                                };

                                // Perform the check
                                match perform_check(&ip.to_string(), &check.spec).await {
                                    Ok(message) => {
                                        // Set state: passing, failures=0
                                        if let Err(e) = redis_manager.set_check_current_state(
                                            &competition_name,
                                            &team.name,
                                            &check.name,
                                            true,
                                            0,
                                            &message,
                                        ) {
                                            error!("Failed to set check state: {}", e);
                                        }
                                        if let Err(e) = redis_manager.record_sucessful_check_result(
                                            &competition_name,
                                            &check.name,
                                            chrono::DateTime::<Utc>::from_timestamp(check_timestamp, 0).expect("Invalid timestamp"),
                                            competition.clone().get_team_id_from_name(&team.name).expect("Team not found"),
                                            &box_config.name,
                                            &message,
                                        ) {
                                            error!("Failed to record check result: {}", e);
                                        }
                                    }
                                    Err(e) => {
                                        // Set state: failing, failures+1
                                        prev_failures += 1;
                                        if let Err(err) = redis_manager.set_check_current_state(
                                            &competition_name,
                                            &team.name,
                                            &check.name,
                                            false,
                                            prev_failures,
                                            &format!("{}", e),
                                        ) {
                                            error!("Failed to set check state: {}", err);
                                        }
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
