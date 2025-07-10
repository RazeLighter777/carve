use anyhow::Result;
use chrono::{DateTime, Utc};
use log::{error, info};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{Duration, sleep};

use crate::check::perform_check;
use carve::config::Competition;
use carve::redis_manager::RedisManager;
use minijinja::{Environment, context};

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
                        let mut messages = Vec::new();
                        let mut passing_boxes = Vec::new();
                        messages.clear();
                        passing_boxes.clear();
                        for box_config in &boxes {
                            // Create an empty HashMap to use as a fallback
                            let empty_selector: HashMap<String, String> = HashMap::new();

                            // Check if this box matches the label selector
                            let label_selector = check
                                .label_selector
                                .as_ref()
                                .or(check.label_selector_alt.as_ref())
                                .unwrap_or(&empty_selector);

                            // If label selector is empty, apply to all boxes
                            // Otherwise, check if box labels match
                            let should_check = label_selector.is_empty()
                                || match label_selector.get("") {
                                    Some(label) => box_config.labels == *label,
                                    None => false,
                                };

                            if should_check {
                                let hostname = format!(
                                    "{}.{}.{}.hack",
                                    box_config.name, team.name, competition_name
                                );
                                // launch dig with cmd to resolve the hostname to an IP address with the vtep's DNS server
                                let ip = match std::process::Command::new("dig")
                                    .arg(&hostname)
                                    .arg("@127.0.0.1")
                                    .arg("+short")
                                    .output()
                                {
                                    Ok(output) if output.status.success() => {
                                        String::from_utf8_lossy(&output.stdout).trim().to_string()
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
                                        println!(
                                            "Box {}.{}.{}.hack has no dns entry (yet), skipping",
                                            box_config.name, team.name, competition_name
                                        );
                                        messages.push(format!(
                                            "Box {}.{}.{}.hack has no dns entry (yet), skipping",
                                            box_config.name, team.name, competition_name
                                        ));

                                        continue;
                                    }
                                };

                                info!(
                                    "Running check {} for team {} on box {} ({})",
                                    check.name, team.name, box_config.name, ip
                                );
                                //record the ip into the redis_manager
                                if let Ok(_) = redis_manager.record_box_ip(
                                    &competition_name,
                                    &team.name,
                                    &box_config.name,
                                    ip,
                                ) {
                                    info!(
                                        "Recorded IP {} for box {}.{}.{}.hack",
                                        ip, box_config.name, team.name, competition_name
                                    );
                                } else {
                                    error!(
                                        "Failed to record IP {} for box {}.{}.{}.hack",
                                        ip, box_config.name, team.name, competition_name
                                    );
                                }

                                // Get box credentials for template substitution
                                let (username, password) = match redis_manager.read_box_credentials(
                                    &competition_name,
                                    &team.name,
                                    &box_config.name,
                                ) {
                                    Ok(Some((u, p))) => (u, p),
                                    _ => ("".to_string(), "".to_string()), // Default empty if not found
                                };

                                // Apply Jinja template substitution to check spec
                                let templated_spec = match apply_template_substitution(
                                    &check.spec,
                                    &team.name,
                                    &box_config.name,
                                    &competition_name,
                                    &ip.to_string(),
                                    &username,
                                    &password,
                                ) {
                                    Ok(spec) => spec,
                                    Err(e) => {
                                        error!("Failed to apply template substitution: {}", e);
                                        continue;
                                    }
                                };

                                // push the message to the messages vector
                                match perform_check(&ip.to_string(), &templated_spec).await {
                                    Ok(message) => {
                                        messages.push(message.clone());
                                        passing_boxes.push(box_config.name.clone());
                                    }
                                    Err(e) => {
                                        // Set state: failing, failures+1
                                        // remove box from passing boxes if it was previously passing
                                        if passing_boxes.contains(&box_config.name) {
                                            passing_boxes.retain(|b| b != &box_config.name);
                                        }
                                        messages.push(format!("{}", e));
                                    }
                                }
                            }
                        }
                        if passing_boxes.is_empty() {
                            info!(
                                "No passing boxes for check {} on team {}",
                                check.name, team.name
                            );
                            continue;
                        } 
                        if let Err(e) = redis_manager.record_sucessful_check_result(
                                &competition_name,
                                &check.name,
                                DateTime::from_timestamp(check_timestamp, 0).expect("Failed to create DateTime"),
                                competition.get_team_id_from_name(&team.name).expect("Team not found"),
                                messages.clone(),
                                passing_boxes.len() as u64,
                        ) {
                            error!("Failed to record successful check result: {}", e);
                        } else {
                            info!(
                                "Recorded successful check result for {} on team {}",
                                check.name, team.name
                            );
                            // get current state of the check so we can get the previous number of failures.
                            let mut prev_failures = 0;
                            if let Ok(Some(current_state)) = redis_manager.get_check_current_state(
                                &competition_name,
                                &team.name,
                                check.name.as_str(),
                            ) {
                                prev_failures = current_state.number_of_failures;
                                info!(
                                    "Current state for check {} on team {}: {:?}",
                                    check.name, team.name, current_state
                                );
                            } else {
                                error!(
                                    "Failed to get current state for check {} on team {}",
                                    check.name, team.name
                                );
                            }
                            // set the current state for the check
                            if let Err(e) = redis_manager.set_check_current_state(
                                &competition_name,
                                &team.name,
                                check.name.as_str(),
                                passing_boxes.len() > 0,
                                if passing_boxes.len() > 0 {
                                    0 // no failures if passing
                                } else {
                                    prev_failures + 1 // increment failures if not passing
                                },
                                messages.clone(),
                                (passing_boxes.len() as u64, messages.len() as u64),
                                passing_boxes.clone(),
                            ) {
                                error!("Failed to set check state: {}", e);
                            } else {
                                info!(
                                    "Set check state for {} on team {} to true",
                                    check.name, team.name
                                );
                            }
                        }

                    }
                }
            });
        }
    }
}

/// Apply Jinja template substitution to check spec string fields
fn apply_template_substitution(
    spec: &carve::config::CheckSpec,
    team_name: &str,
    box_name: &str,
    competition_name: &str,
    ip_address: &str,
    username: &str,
    password: &str,
) -> Result<carve::config::CheckSpec, anyhow::Error> {
    use carve::config::CheckSpec;

    // Create context with all available variables
    let template_context = context! {
        team_name => team_name,
        box_name => box_name,
        competition_name => competition_name,
        ip_address => ip_address,
        username => username,
        password => password
    };

    match spec {
        CheckSpec::Http(http_spec) => {
            // Apply templating to HTTP check fields
            let url = apply_template_to_string(&http_spec.url, &template_context)?;
            let regex = apply_template_to_string(&http_spec.regex, &template_context)?;

            Ok(CheckSpec::Http(carve::config::HttpCheckSpec {
                url,
                code: http_spec.code,
                regex,
            }))
        }
        CheckSpec::Icmp(icmp_spec) => {
            // ICMP spec has no string fields to template
            Ok(CheckSpec::Icmp(icmp_spec.clone()))
        }
        CheckSpec::Ssh(ssh_spec) => {
            // Apply templating to SSH check fields
            let username = apply_template_to_string(&ssh_spec.username, &template_context)?;
            let password = ssh_spec
                .password
                .as_ref()
                .map(|p| apply_template_to_string(p, &template_context))
                .transpose()?;
            let key_path = ssh_spec
                .key_path
                .as_ref()
                .map(|p| apply_template_to_string(p, &template_context))
                .transpose()?;

            Ok(CheckSpec::Ssh(carve::config::SshCheckSpec {
                port: ssh_spec.port,
                username,
                password,
                key_path,
            }))
        }
    }
}

/// Apply Jinja template to a single string field
fn apply_template_to_string(
    template_str: &str,
    context: &minijinja::Value,
) -> Result<String, anyhow::Error> {
    // Create a fresh environment for each template to avoid lifetime issues
    let mut env = Environment::new();

    // Create a unique template name based on the content hash to avoid conflicts
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    template_str.hash(&mut hasher);
    let template_name = format!("tmpl_{}", hasher.finish());

    // Add template to environment
    env.add_template(&template_name, template_str)?;

    // Get template and render
    let tmpl = env.get_template(&template_name)?;
    let rendered = tmpl.render(context)?;

    Ok(rendered)
}
