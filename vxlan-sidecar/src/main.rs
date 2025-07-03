use actix_web::{App, HttpServer};
use carve::config::AppConfig;
use carve::redis_manager::RedisManager;
use std::env;
use std::net::IpAddr;
use std::process::Command;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, sleep};

fn create_vxlan_interface(vxlan_id: &str) -> Result<(), String> {
    // Remove vxlan0 if it exists
    let _ = Command::new("ip").args(["link", "del", "vxlan0"]).status();
    // Create vxlan0 with remote
    let status = Command::new("ip")
        .args([
            "link", "add", "vxlan0", "type", "vxlan", "id", vxlan_id, "dev", "eth0", "nolearning",
            "dstport", "4789",
        ])
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

fn create_bridge() -> Result<(), String> {
    // Remove br0 if it exists
    let _ = Command::new("ip").args(["link", "del", "br0"]).status();
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
    Command::new("ip")
        .args(["link", "set", "vxlan0", "master", "br0"])
        .status()
        .map_err(|e| format!("Failed to add vxlan0 to br0: {}", e))?;
    // we don't need to set an ip address on vxlan0, as it will be used for bridging only
    // // Add IP address to br0
    // // Set vxlan0 address to .254 of the given CIDR
    // let ip_address = format!("{}/24", cidr.trim_end_matches(".0").to_string() + ".254");
    // // println!("Setting br0 address to {}", ip_address);
    // Command::new("ip")
    //     .args(["addr", "add", &ip_address, "dev", "br0"])
    //     .status()
    //     .map_err(|e| format!("Failed to set br0 address: {}", e))?;
    Ok(())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let team_name = env::var("TEAM_NAME").expect("TEAM_NAME env var required");
    let config = AppConfig::new().expect("Failed to load config");
    let competition_name = env::var("COMPETITION_NAME").expect("COMPETITION_NAME env var required");
    let box_name = env::var("BOX_NAME").expect("BOX_NAME env var required");
    let competition = config
        .competitions
        .iter()
        .find(|c| c.name == competition_name)
        .expect(
            format!(
                "Competition {} not found in config",
                competition_name.as_str()
            )
            .as_str(),
        )
        .clone();
    let teams: Vec<String> = competition.teams.iter().map(|t| t.name.clone()).collect();
    let team_index = teams
        .iter()
        .position(|n| n == &team_name)
        .expect("TEAM_NAME not found in config");
    let vxlan_id = 1338 + team_index as u32;
    // Use vtep_host from the first competition, fallback to localhost if missing
    if let Err(e) = create_vxlan_interface(&vxlan_id.to_string()) {
        eprintln!("{}", e);
    }
    if let Err(e) = create_bridge() {
        eprintln!("{}", e);
    }

    // // Add route for the competitions's /16 subnet via .1 (first IP in team /24 subnet) on br0
    // let route_command = Command::new("ip")
    //     .args([
    //         "route",
    //         "add",
    //         &format!("{}", competition.cidr.as_ref().unwrap()),
    //         "via",
    //         &format!("{}.{}.{}.1", octets[0], octets[1], octets[2]),
    //         "dev",
    //         "br0",
    //     ])
    //     .status();
    // match route_command {
    //     Ok(status) if status.success() => {
    //         println!("Route added successfully for competition subnet");
    //     }
    //     Ok(_) | Err(_) => {
    //         eprintln!("Failed to add route for competition subnet");
    //     }
    // }

    // DNS resolution loop for vtep_host
    let remote = competition.vtep_host.as_deref().unwrap_or("127.0.0.1");
    let vtep_host = remote.to_string();
    let last_ip = Arc::new(Mutex::new(None::<IpAddr>));
    let last_ip_clone = last_ip.clone();
    print!(
        "Starting DNS resolution loop for VTEP host: {}\n",
        vtep_host
    );
    tokio::spawn(async move {
        let competition = competition.clone();
        let redis_manager = RedisManager::new(&competition.redis).unwrap();

        loop {
            let new_fdb_entries: Vec<(String, String)> = redis_manager
                .get_domain_fdb_entries(&competition_name, &team_name)
                .unwrap();
            for entry in new_fdb_entries {
                let (mac, ip) = entry;
                // skip entry with the ip of the vxlan-sidecar-<our_team_name>-<our_box_name> service
                match tokio::net::lookup_host((
                    format!("vxlan-sidecar-{}-{}", team_name, box_name).as_str(),
                    0,
                ))
                .await
                {
                    Ok(mut addrs) => {
                        if let Some(vxlan_ip) = addrs.next().map(|sockaddr| sockaddr.ip()) {
                            println!("entry ip: {} and our vxlan ip: {}", ip, vxlan_ip);
                            if vxlan_ip.to_string() == ip {
                                println!(
                                    "Skipping FDB entry for vxlan-sidecar-{}-{}: {} {}",
                                    team_name, box_name, mac, ip
                                );
                                continue;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!(
                            "DNS resolution error for vxlan-sidecar-{}-{}: {}",
                            team_name, box_name, e
                        );
                    }
                }
                let status1 = Command::new("bridge")
                    .args([
                        "fdb",
                        "append",
                        "00:00:00:00:00:00",
                        "dst",
                        &ip,
                        "dev",
                        "vxlan0",
                        "dynamic",
                    ])
                    .status();
                match status1 {
                    Ok(s) if s.success() => {
                        println!(
                            "Appended multicast FDB entry for vxlan0 remote {} with MAC {}",
                            ip, mac
                        );
                    }
                    Ok(_) | Err(_) => {
                        eprintln!(
                            "Failed to append multicast FDB entry for vxlan0 remote {} with MAC {}",
                            ip, mac
                        );
                    }
                }
                let status2 = Command::new("bridge")
                    .args([
                        "fdb",
                        "append",
                        &mac,
                        "dst",
                        &ip,
                        "dev",
                        "vxlan0",
                        "dynamic",
                    ])
                    .status();
                match status2 {
                    Ok(s) if s.success() => {
                        println!(
                            "Appended FDB entry for vxlan0 remote {} with MAC {}",
                            ip, mac
                        ); 
                    }
                    Ok(_) | Err(_) => {
                        eprintln!(
                            "Failed to append FDB entry for vxlan0 remote {} with MAC {}",
                            ip, mac
                        );
                    }
                }
            }
            // get mac address and ip of eth0 and add it to the FDB
            //now lookup the ip of the vxlan-sidecar-<team_name>-<box_name> service and use redis manager to publish the FDB entry

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
                                .args([
                                    "fdb",
                                    "append",
                                    "00:00:00:00:00:00",
                                    "dst",
                                    &ip.to_string(),
                                    "dev",
                                    "vxlan0",
                                ])
                                .status();
                            match status {
                                Ok(s) if s.success() => {
                                    *last = Some(ip);
                                    println!("Appended FDB entry for vxlan0 remote {}", ip);
                                }
                                Ok(_) | Err(_) => {
                                    eprintln!(
                                        "Failed to append FDB entry for vxlan0 remote {}",
                                        ip
                                    );
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

    HttpServer::new(move || App::new())
        .bind(("0.0.0.0", 8000))?
        .run()
        .await
}
