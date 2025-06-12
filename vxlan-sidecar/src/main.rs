use actix_web::{get, App, HttpServer, Responder, web};
use carve::config::AppConfig;
use std::env;
use std::process::Command;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use std::net::IpAddr;

#[derive(Clone)]
enum HealthStatus {
    Healthy,
    Unhealthy,
}

type SharedHealth = Arc<Mutex<HealthStatus>>;

#[get("/api/health")]
async fn health(health: web::Data<SharedHealth>) -> impl Responder {
    let status = health.lock().await;
    match *status {
        HealthStatus::Healthy => "Healthy",
        HealthStatus::Unhealthy => "Unhealthy",
    }
}

fn create_vxlan_interface(vxlan_id: &str) -> Result<(), String> {
    // Remove vxlan0 if it exists
    let _ = Command::new("ip")
        .args(["link", "del", "vxlan0"])
        .status();
    // Create vxlan0 with remote
    let status = Command::new("ip")
        .args(["link", "add", "vxlan0", "type", "vxlan", "id", vxlan_id, "dev", "eth0", "dstport", "4789"])
        .status()
        .map_err(|e| format!("Failed to create vxlan0: {}", e))?;
    if !status.success() {
        return Err("Failed to create vxlan0 interface".into());
    }
    // Bring up vxlan0
    Command::new("ip")
        .args(["link", "set", "vxlan0", "up"])
        .status()
        .map_err(|e| format!("Failed to bring up vxlan0: {}", e))?;
    Ok(())
}

fn create_bridge(cidr : &str) -> Result<(), String> {
    // Remove br0 if it exists
    let _ = Command::new("ip")
        .args(["link", "del", "br0"])
        .status();
    // Create br0
    let status = Command::new("ip")
        .args(["link", "add", "name", "br0", "type", "bridge"])
        .status()
        .map_err(|e| format!("Failed to create br0: {}", e))?;
    if !status.success() {
        return Err("Failed to create br0 interface".into());
    }
    // Bring up br0
    Command::new("ip")
        .args(["link", "set", "br0", "up"])
        .status()
        .map_err(|e| format!("Failed to bring up br0: {}", e))?;
    // Add vxlan0 to br0
    Command::new("ip")
        .args(["link", "set", "vxlan0", "master", "br0"])
        .status()
        .map_err(|e| format!("Failed to add vxlan0 to br0: {}", e))?;
    // Add IP address to br0
    // Set vxlan0 address to .254 of the given CIDR
    let ip_address = format!("{}/24", cidr.trim_end_matches(".0").to_string() + ".254");
    Command::new("ip")
        .args(["addr", "add", &ip_address, "dev", "br0"])
        .status()
        .map_err(|e| format!("Failed to set br0 address: {}", e))?;
    Ok(())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let team_name = env::var("TEAM_NAME").expect("TEAM_NAME env var required");
    let config = AppConfig::new().expect("Failed to load config");
    let competition = config.competitions.get(0).expect("No competition in config");
    let teams: Vec<String> = competition.teams.iter().map(|t| t.name.clone()).collect();
    let team_index = teams.iter().position(|n| n == &team_name).expect("TEAM_NAME not found in config");
    let vxlan_id = 1338 + team_index as u32;

    // get the cidr for the team
    let cidr = competition.cidr.as_ref().expect("Competition CIDR missing");
    let base = cidr.split('/').next().expect("Invalid CIDR");
    let mut octets: Vec<u8> = base.split('.').map(|s| s.parse().unwrap()).collect();
    // Set third octet to team_index+1
    octets[2] = (team_index + 1) as u8;
    octets[3] = 0;
    let team_cidr = format!("{}.{}.{}.0", octets[0], octets[1], octets[2]);


    // Use vtep_host from the first competition, fallback to localhost if missing
    if let Err(e) = create_vxlan_interface(&vxlan_id.to_string()) {
        eprintln!("{}", e);
    }
    if let Err(e) = create_bridge(&team_cidr) {
        eprintln!("{}", e);
    }

    // Add route for the competitions's /16 subnet via .1 (first IP in team /24 subnet) on br0
    let route_command = Command::new("ip")
        .args(["route", "add", &format!("{}", competition.cidr.as_ref().unwrap()), "via", &format!("{}.{}.{}.1", octets[0], octets[1], octets[2]), "dev", "br0"])
        .status();
    match route_command {
        Ok(status) if status.success() => {
            println!("Route added successfully for competition subnet");
        }
        Ok(_) | Err(_) => {
            eprintln!("Failed to add route for competition subnet");
        }
    }

    // DNS resolution loop for vtep_host
    let remote = competition.vtep_host.as_deref().unwrap_or("127.0.0.1");
    let vtep_host = remote.to_string();
    let last_ip = Arc::new(Mutex::new(None::<IpAddr>));
    let last_ip_clone = last_ip.clone();
    tokio::spawn(async move {
        loop {
            match tokio::net::lookup_host((vtep_host.as_str(), 0)).await {
                Ok(mut addrs) => {
                    if let Some(ip) = addrs.next().map(|sockaddr| sockaddr.ip()) {
                        let mut last = last_ip_clone.lock().await;
                        if last.map_or(true, |prev| prev != ip) {
                            // flush the FDB entries for vxlan0
                            let _ = Command::new("bridge")
                                .args(["fdb", "flush", "dev", "vxlan0"])
                                .status();
                            // Only update if IP changed
                            let status = Command::new("bridge")
                                .args(["fdb", "append", "00:00:00:00:00:00", "dst", &ip.to_string(), "dev", "vxlan0"])
                                .status();
                            match status {
                                Ok(s) if s.success() => {
                                    *last = Some(ip);
                                    println!("Appended FDB entry for vxlan0 remote {}", ip);
                                }
                                Ok(_) | Err(_) => {
                                    eprintln!("Failed to append FDB entry for vxlan0 remote {}", ip);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("DNS resolution error for {}: {}", vtep_host, e);
                }
            }
            sleep(Duration::from_secs(10)).await;
        }
    });

    // Health check state
    let health_status: SharedHealth = Arc::new(Mutex::new(HealthStatus::Healthy));
    let health_status_clone = health_status.clone();
    let first_ip = format!("{}.{}.{}.1", octets[0], octets[1], octets[2]);
    tokio::spawn(async move {
        let mut fail_count = 0;
        loop {
            let output = Command::new("ping")
                .args(["-c", "1", "-W", "2", &first_ip])
                .output();
            match output {
                Ok(out) if out.status.success() => {
                    if fail_count > 3 {
                        println!("Ping to {} successful after {} failures", first_ip, fail_count);
                    }
                    fail_count = 0;
                    let mut status = health_status_clone.lock().await;
                    *status = HealthStatus::Healthy;
                    
                },
                _ => {
                    fail_count += 1;
                    if fail_count >= 3 {
                        let mut status = health_status_clone.lock().await;
                        *status = HealthStatus::Unhealthy;
                        eprintln!("Ping to {} failed {} times, marking as Unhealthy", first_ip, fail_count);
                    }
                    
                }
            }
            sleep(Duration::from_secs(10)).await;
        }
    });

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(health_status.clone()))
            .service(health)
    })
    .bind(("0.0.0.0", 8000))?
    .run()
    .await
}
