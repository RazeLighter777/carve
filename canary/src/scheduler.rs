use anyhow::Result;
use chrono::{DateTime, Utc};
use log::{error, info, debug};
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
        debug!("Creating new Scheduler for competition: {}", competition.name);
        Self {
            competition,
            redis_manager,
        }
    }

    async fn preload_nix_checks(competition: &Competition) {
        debug!("Preloading Nix checks for competition: {}", competition.name);
        for check in &competition.checks {
            if let carve::config::CheckSpec::Nix(nix_check) = &check.spec {
                // Preload Nix checks by running nix-shell -p with all the packages
                let default = vec!["nixpkgs".into()];
                let packages = nix_check.packages.as_ref().unwrap_or(&default);
                let output = tokio::process::Command::new("nix-shell")
                    .arg("-p")
                    .args(packages.iter().map(String::as_str))
                    .output()
                    .await;
                match output {
                    Ok(output) if output.status.success() => {
                        info!("Preloaded Nix check {} with packages: {:?}", check.name, packages);
                    }
                    Ok(output) => {
                        error!("Failed to preload Nix check {}: {}", check.name, String::from_utf8_lossy(&output.stderr));
                    }
                    Err(e) => {
                        error!("Error preloading Nix check {}: {}", check.name, e);
                    }
                }
            }
        }
    }

    pub async fn run(self) {
        debug!("Starting scheduler run for competition: {}", self.competition.name);
        Self::preload_nix_checks(&self.competition).await;
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

                    // Set timeout to 80% of interval
                    let team_timeout = Duration::from_secs((check.interval as f64 * 0.8) as u64);
                    let mut handles = Vec::new();

                    for team in &teams {
                        let team = team.clone();
                        let boxes = boxes.clone();
                        let check = check.clone();
                        let competition = competition.clone();
                        let redis_manager = redis_manager.clone();
                        let competition_name = competition_name.clone();
                        let check_timestamp = check_timestamp;
                        let handle = tokio::spawn(async move {
                            use tokio::task::JoinSet;
                            let mut set = JoinSet::new();
                            for box_config in &boxes {
                                let box_config = box_config.clone();
                                let check = check.clone();
                                let team = team.clone();
                                let redis_manager = redis_manager.clone();
                                let competition_name = competition_name.clone();
                                set.spawn(async move {
                                    let empty_selector: HashMap<String, String> = HashMap::new();
                                    let label_selector = check
                                        .label_selector
                                        .as_ref()
                                        .or(check.label_selector_alt.as_ref())
                                        .unwrap_or(&empty_selector);
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
                                                return (None, None);
                                            }
                                        };
                                        // check if we got a valid IP address
                                        let ip = match ip.parse::<std::net::IpAddr>() {
                                            Ok(ip) => ip,
                                            Err(_) => {
                                                let msg = format!(
                                                    "Box {}.{}.{}.hack has no dns entry (yet), skipping",
                                                    box_config.name, team.name, competition_name
                                                );
                                                info!("{}", msg);
                                                return (Some(msg), None);
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
                                                return (Some(format!("Failed to apply template: {}", e)), None);
                                            }
                                        };

                                        // push the message to the messages vector
                                        match perform_check(&ip.to_string(), &templated_spec).await {
                                            Ok(message) => {
                                                return (Some(message), Some(box_config.name.clone()));
                                            }
                                            Err(e) => {
                                                return (Some(format!("{}", e)), None);
                                            }
                                        }
                                    }
                                    (None, None)
                                });
                            }
                            let mut messages = Vec::new();
                            let mut passing_boxes = Vec::new();
                            while let Some(res) = set.join_next().await {
                                if let Ok((msg_opt, passing_opt)) = res {
                                    if let Some(msg) = msg_opt {
                                        messages.push(msg);
                                    }
                                    if let Some(box_name) = passing_opt {
                                        passing_boxes.push(box_name);
                                    }
                                }
                            }
                            if passing_boxes.is_empty() {
                                info!(
                                    "No passing boxes for check {} on team {}",
                                    check.name, team.name
                                );
                                debug!("Messages for failed check: {:?}", messages);
                                return;
                            }
                            if let Err(e) = redis_manager.record_sucessful_check_result(
                                    &competition_name,
                                    &check.name,
                                    DateTime::from_timestamp(check_timestamp, 0).expect("Failed to create DateTime"),
                                    competition.get_team_id_from_name(&team.name).expect("Team not found"),
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
                        });
                        handles.push(handle);
                    }

                    // Wait for all team tasks to finish, aborting those that take too long
                    for handle in handles {
                        match tokio::time::timeout(team_timeout, handle).await {
                            Ok(res) => {
                                let _ = res;
                            }
                            Err(_) => {
                                error!("Team check task timed out and could not be aborted (handle moved)");
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
                method: http_spec.method.clone(),
                forms: http_spec.forms.as_ref().map(|f| {
                    apply_template_to_string(f, &template_context).unwrap_or_default()
                }),
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
        CheckSpec::Nix(nix_spec) => {
            // Apply templating to Nix check script
            let script = apply_template_to_string(&nix_spec.script, &template_context)?;
            debug!("Nix script after templating: {}", script);
            Ok(CheckSpec::Nix(carve::config::NixCheckSpec { script, packages: nix_spec.packages.clone(), timeout: nix_spec.timeout }))
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
