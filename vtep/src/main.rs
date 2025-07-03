use actix_web::{App, HttpServer, Responder, get};
use carve::{config::AppConfig, redis_manager};
use redis::Commands;
use std::collections::HashMap;
use std::net::{Ipv4Addr, ToSocketAddrs};

#[get("/health")]
async fn health() -> impl Responder {
    "Healthy"
}

fn parse_cidr(cidr: &str) -> (Ipv4Addr, u8) {
    let parts: Vec<&str> = cidr.split('/').collect();
    let ip = parts[0].parse().expect("Invalid IP in CIDR");
    let prefix = parts[1].parse().expect("Invalid prefix in CIDR");
    (ip, prefix)
}

fn allocate_subnets(base: Ipv4Addr, prefix: u8, num: usize) -> Vec<Ipv4Addr> {
    let mut subnets = Vec::new();
    let step = 1 << (32 - (prefix + 8)); // /24s from /16
    let mut current = u32::from(base);
    for _ in 0..num {
        subnets.push(Ipv4Addr::from(current));
        current += step;
    }
    subnets
}

fn main() {
    // Load config
    let config = AppConfig::new().expect("Failed to load config");
    for competition in &config.competitions.clone() {
        // Get CIDR from config (add to schema if missing)
        let cidr = competition.cidr.as_ref().expect("competition.cidr missing");
        let (base, prefix) = parse_cidr(cidr);
        let num_teams = competition.teams.len();
        let mut subnets = allocate_subnets(base, prefix, num_teams + 1); // +1 for MGMT
        let mgmt_subnet = subnets.remove(0);
        // Connect to redis
        let redis_url = format!(
            "redis://{}:{}/{}",
            competition.redis.host, competition.redis.port, competition.redis.db
        );
        let client = redis::Client::open(redis_url).expect("redis client");
        let mut con = client.get_connection().expect("redis conn");
        // Clean subnets hash
        let _: () = redis::cmd("DEL")
            .arg(format!("{}:subnets", competition.name))
            .query(&mut con)
            .unwrap();
        // Allocate subnets and VXLAN IDs
        let mut vxlan_id = 1338u32;
        let mut subnet_map = HashMap::new();
        subnet_map.insert("MGMT".to_string(), format!("{}/24,MGMT,0", mgmt_subnet));
        for (team, subnet) in competition.teams.iter().zip(subnets.iter()) {
            subnet_map.insert(
                team.name.clone(),
                format!("{}/24,{},{}", subnet, team.name, vxlan_id),
            );
            vxlan_id += 1;
        }
        // Store in redis
        let _: () = con
            .hset_multiple(
                format!("{}:subnets", competition.name),
                &subnet_map
                    .iter()
                    .map(|(k, v)| (k.as_str(), v.as_str()))
                    .collect::<Vec<_>>(),
            )
            .unwrap();
        // Set up iptables
        let ipt = iptables::new(false).expect("Failed to create iptables instance");
        // Create MGMT VXLAN interface
        // Use a short index for interface names to avoid long names
        let comp_idx = config
            .competitions
            .iter()
            .position(|c| c.name == competition.name)
            .unwrap_or(0);
        let vxlan_mgmt_name = format!("vxlan_mgmt_{}", comp_idx);
        let vxlan_mgmt_id = 1337; // MGMT VXLAN ID is 1337
        let status = std::process::Command::new("ip")
            .args([
                "link",
                "add",
                &vxlan_mgmt_name,
                "type",
                "vxlan",
                "id",
                &vxlan_mgmt_id.to_string(),
                "dev",
                "eth0",
                "nolearning",
                "dstport",
                "4789",
            ])
            .status()
            .expect("Failed to create vxlan interface");
        if !status.success() {
            eprintln!("Failed to create vxlan interface {}", vxlan_mgmt_name);
        }
        // Bring up the MGMT VXLAN interface
        std::process::Command::new("ip")
            .args(["link", "set", &vxlan_mgmt_name, "up"])
            .status()
            .expect("Failed to bring up vxlan interface");
        // Assign MGMT subnet IP to interface (first IP in subnet)
        let mgmt_gateway_ip = Ipv4Addr::from(u32::from(mgmt_subnet) + 1);
        std::process::Command::new("ip")
            .args([
                "addr",
                "add",
                &format!("{}/24", mgmt_gateway_ip),
                "dev",
                &vxlan_mgmt_name,
            ])
            .status()
            .expect("Failed to assign IP to vxlan interface");

        // Create VXLAN interfaces for each team and SNAT their traffic
        for (i, team) in competition.teams.iter().enumerate() {
            let vxlan_name = format!("vxlan_{}_{}", comp_idx, i);
            let vxlan_id = 1338 + i as u32; // Start VXLAN IDs from 1338
            let team_subnet = subnets.get(i).expect("subnet");
            println!(
                "Creating vxlan interface for {} named {}",
                team.name, vxlan_name
            );
            // Remove interface if it exists
            let _ = std::process::Command::new("ip")
                .args(["link", "del", &vxlan_name])
                .status();
            // Create VXLAN interface
            let status = std::process::Command::new("ip")
                .args([
                    "link",
                    "add",
                    &vxlan_name,
                    "type",
                    "vxlan",
                    "id",
                    &vxlan_id.to_string(),
                    "dev",
                    "eth0",
                    "nolearning",
                    "dstport",
                    "4789",
                ])
                .status()
                .expect("Failed to create vxlan interface");
            if !status.success() {
                eprintln!("Failed to create vxlan interface {}", vxlan_name);
            }
            // create a bridge for the VXLAN interface
            let bridge_name = format!("br_{}_{}", comp_idx, i);
            let status = std::process::Command::new("ip")
                .args(["link", "add", &bridge_name, "type", "bridge"])
                .status()
                .expect("Failed to create bridge interface");
            if !status.success() {
                eprintln!("Failed to create bridge interface {}", bridge_name);
            }
            // Add the VXLAN interface to the bridge
            std::process::Command::new("ip")
                .args(["link", "set", &vxlan_name, "master", &bridge_name])
                .status()
                .expect("Failed to add vxlan interface to bridge");
            // Bring up the bridge interface
            std::process::Command::new("ip")
                .args(["link", "set", &bridge_name, "up"])
                .status()
                .expect("Failed to bring up bridge interface");
            // Bring up the interface
            std::process::Command::new("ip")
                .args(["link", "set", &vxlan_name, "up"])
                .status()
                .expect("Failed to bring up vxlan interface");
            // Assign subnet IP to bridge interface (first IP in subnet)
            let team_gateway_ip = Ipv4Addr::from(u32::from(*team_subnet) + 1);
            std::process::Command::new("ip")
                .args([
                    "addr",
                    "add",
                    &format!("{}/24", team_gateway_ip),
                    "dev",
                    &bridge_name,
                ])
                .status()
                .expect("Failed to assign IP to bridge interface");
            // SNAT only traffic from this team's VXLAN interface, using MGMT /24 as --to-source
            let team_snat_rule = format!("-o {} -j MASQUERADE", bridge_name);
            ipt.append("nat", "POSTROUTING", &team_snat_rule)
                .expect("Failed to add SNAT rule for team");
            // mangle TTL to 64 for the VXLAN interface to stop teams from
            // using it to block other teams and allow the scoring server to reach them
            let team_ttl_rule = format!("-i {} -j TTL --ttl-set 64", bridge_name);
            ipt.append("mangle", "PREROUTING", &team_ttl_rule)
                .expect("Failed to add TTL rule for team");
        }
        // print iptables command for debugging
    }
    // Execute the firewall rules

    // Start Actix-web server for health check
    std::thread::spawn(|| {
        let sys = actix_rt::System::new();
        sys.block_on(async {
            HttpServer::new(|| App::new().service(health))
                .bind(("0.0.0.0", 8000))
                .expect("Failed to bind Actix server")
                .run()
                .await
                .ok();
        });
    });
    let config2 = config.clone();
    //start fdb update thread
    std::thread::spawn(move || {
        // create static fdb entry for 00:00:00:00:00:00 for each of the vxlan-sidecar-<team_name>-<box_name> containers
        loop {
            for (i, config) in (&config2).clone().competitions.iter().enumerate() {
                for (j, team) in config.teams.iter().enumerate() {
                    let redis_manager = redis_manager::RedisManager::new(&config.redis)
                        .expect("Failed to create Redis manager");

                    let mac_address = std::process::Command::new("cat")
                        .arg(format!("/sys/class/net/vxlan_{}_{}//address", i, j))
                        .output()
                        .expect("Failed to get MAC address")
                        .stdout;
                    let mac_address = String::from_utf8(mac_address)
                        .expect("Failed to convert MAC address to string")
                        .trim()
                        .to_string();
                    // now lookup our IP address from the vtep-host in the config
                    let host = format!(
                        "{}:4789",
                        &config.clone().vtep_host.expect("vtep_host missing")
                    );

                    match host.to_socket_addrs() {
                        Ok(mut addrs) => {
                            if let Some(addr) = addrs.next() {
                                match redis_manager.create_vxlan_fdb_entry(
                                    &config.name,
                                    &mac_address,
                                    addr.ip(),
                                    team.name.as_str(),
                                ) {
                                    Ok(_) => {
                                        println!(
                                            "Published FDB entry for {}: {} -> {}",
                                            team.name.as_str(),
                                            addr.ip(),
                                            mac_address
                                        );
                                    }
                                    Err(e) => {
                                        eprintln!(
                                            "Failed to publish FDB entry for {}: {}",
                                            team.name.as_str(),
                                            e
                                        );
                                    }
                                }
                            } else {
                                eprintln!("No addresses found for {}", host);
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to resolve {}: {}", host, e);
                        }
                    }
                    for b in &config.boxes {
                        let broadcast_entry = "00:00:00:00:00:00".to_string();
                        let redis_fdb_entries = redis_manager
                            .get_domain_fdb_entries(&config.name, &team.name)
                            .expect("Failed to get FDB entries");
                        for (mac, addr) in redis_fdb_entries {
                            // Add the FDB entry to the bridge
                            println!("mac: {}, addr: {}", mac, addr);
                            let vxlan_name = format!("vxlan_{}_{}", i, j);
                            println!(
                                "Adding FDB entries for  this {}: {} -> {}",
                                vxlan_name, mac, addr
                            );
                            let vxlan_sidecar_name =
                                format!("vxlan-sidecar-{}-{}", team.name, b.name);
                            let status = std::process::Command::new("bridge")
                                .args([
                                    "fdb",
                                    "append",
                                    &mac,
                                    "dev",
                                    &vxlan_name,
                                    "dst",
                                    &addr,
                                    "dynamic",
                                ])
                                .status()
                                .expect("Failed to add FDB entry");
                            if !status.success() {
                                eprintln!(
                                    "Failed to add unicast FDB entry for {}: {}",
                                    vxlan_sidecar_name, status
                                );
                            } else {
                                println!(
                                    "Added unicast FDB entry for {}: {} -> {}",
                                    vxlan_sidecar_name, mac, addr
                                );
                            }
                            // Add the broadcast FDB entry for the vxlan-sidecar
                            let status = std::process::Command::new("bridge")
                                .args([
                                    "fdb",
                                    "append",
                                    &broadcast_entry,
                                    "dev",
                                    &vxlan_name,
                                    "dst",
                                    &addr,
                                    "dynamic",
                                ])
                                .status()
                                .expect("Failed to add broadcast FDB entry");
                            if !status.success() {
                                eprintln!(
                                    "Failed to add broadcast FDB entry for {}: {}",
                                    vxlan_sidecar_name, status
                                );
                            } else {
                                println!(
                                    "Added broadcast FDB entry for {}: {} -> {}",
                                    vxlan_sidecar_name, mac, broadcast_entry
                                );
                            }
                        }
                    }
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(10));
        }
        // Sleep for a while before updating again
    });

    // subscribe to <competition_name>:events
    // NAT outgoing traffic to the internet ONLY if the competition is NOT running
    let redis_manager = redis_manager::RedisManager::new(&config.competitions[0].redis)
        .expect("Failed to create Redis manager");
    let ipt = iptables::new(false).expect("Failed to create iptables instance");
    let mut rule_added = false;
    let current_competition_state = redis_manager
        .get_competition_state(config.competitions[0].name.as_str())
        .expect("Failed to get competition state");
    match current_competition_state.status {
        redis_manager::CompetitionStatus::Unstarted => {
            // NAT outgoing traffic to the internet
            println!("Competition is not running, enabling NAT for outgoing traffic");
            // Add NAT rule for outgoing traffic
            let nat_rule = "-o eth0 -j MASQUERADE";
            ipt.append("nat", "POSTROUTING", nat_rule)
                .expect("Failed to add NAT rule for outgoing traffic");
            rule_added = true;
        }
        redis_manager::CompetitionStatus::Active => {
            // do nothing, rule doesn't need to be added
            println!("Competition is running, NAT disabled for outgoing traffic");
        }
        redis_manager::CompetitionStatus::Finished => {
            // Do nothing, competition is finished
        }
    }
    loop {
        match redis_manager.wait_for_competition_event(config.competitions[0].name.as_str()) {
            Ok(event) => {
                match event.status {
                    redis_manager::CompetitionStatus::Unstarted => {
                        // NAT outgoing traffic to the internet
                        println!("Competition is not running, enabling NAT for outgoing traffic");
                        // Add NAT rule for outgoing traffic
                        let nat_rule = "-o eth0 -j MASQUERADE";
                        if !rule_added {
                            // Only add the rule if it wasn't added before
                            ipt.append("nat", "POSTROUTING", nat_rule)
                                .expect("Failed to add NAT rule for outgoing traffic");
                            rule_added = true;
                        }
                    }
                    redis_manager::CompetitionStatus::Active => {
                        // Disable NAT for outgoing traffic
                        println!("Competition is running, disabling NAT for outgoing traffic");
                        // Remove NAT rule for outgoing traffic
                        let nat_rule = "-o eth0 -j MASQUERADE";
                        if rule_added {
                            ipt.delete("nat", "POSTROUTING", nat_rule)
                                .expect("Failed to remove NAT rule for outgoing traffic");
                        }
                    }
                    redis_manager::CompetitionStatus::Finished => {
                        // Do nothing, competition is finished
                    }
                }
            }
            Err(e) => {
                eprintln!("Error waiting for competition event: {}", e);
            }
        }
    }
}
