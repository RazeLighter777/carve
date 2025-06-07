use carve::config::{AppConfig};
use iptables;
use redis::Commands;
use std::collections::HashMap;
use std::net::Ipv4Addr;
use actix_web::{App, HttpServer, Responder, get};

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
    for competition in &config.competitions {
        // Get CIDR from config (add to schema if missing)
        let cidr = competition.cidr.as_ref().expect("competition.cidr missing");
        let (base, prefix) = parse_cidr(cidr);
        let num_teams = competition.teams.len();
        let mut subnets = allocate_subnets(base, prefix, num_teams + 1); // +1 for MGMT
        let mgmt_subnet = subnets.remove(0);
        // Connect to redis
        let redis_url = format!("redis://{}:{}/{}", competition.redis.host, competition.redis.port, competition.redis.db);
        let client = redis::Client::open(redis_url).expect("redis client");
        let mut con = client.get_connection().expect("redis conn");
        // Clean subnets hash
        let _: () = redis::cmd("DEL").arg(format!("{}:subnets", competition.name)).query(&mut con).unwrap();
        // Allocate subnets and VXLAN IDs
        let mut vxlan_id = 1u32;
        let mut subnet_map = HashMap::new();
        subnet_map.insert("MGMT".to_string(), format!("{}/24,MGMT,0", mgmt_subnet));
        for (team, subnet) in competition.teams.iter().zip(subnets.iter()) {
            subnet_map.insert(
                team.name.clone(),
                format!("{}/24,{},{}", subnet, team.name, vxlan_id)
            );
            vxlan_id += 1;
        }
        // Store in redis
        let _: () = con.hset_multiple(
            format!("{}:subnets", competition.name),
            &subnet_map.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect::<Vec<_>>()
        ).unwrap();
        // Set up iptables
        let ipt = iptables::new(false).expect("Failed to create iptables instance");        
        // Enable IPv4 forwarding
        std::process::Command::new("sysctl")
            .arg("-w")
            .arg("net.ipv4.ip_forward=1")
            .status()
            .expect("Failed to enable IPv4 forwarding");
        // Create VXLAN interfaces for each team and SNAT their traffic
        for (i, team) in competition.teams.iter().enumerate() {
            let vxlan_name = format!("vxlan_{}_{}", team.name, i + 1);
            let vxlan_id = i + 1;
            let team_subnet = subnets.get(i).expect("subnet");
            // Remove interface if it exists
            let _ = std::process::Command::new("ip")
                .args(["link", "del", &vxlan_name])
                .status();
            // Create VXLAN interface
            let status = std::process::Command::new("ip")
                .args(["link", "add", &vxlan_name, "type", "vxlan", "id", &vxlan_id.to_string(), "dev", "eth0", "learning"]) // assumes eth0
                .status()
                .expect("Failed to create vxlan interface");
            if !status.success() {
                eprintln!("Failed to create vxlan interface {}", vxlan_name);
            }
            // Bring up the interface
            std::process::Command::new("ip")
                .args(["link", "set", &vxlan_name, "up"])
                .status()
                .expect("Failed to bring up vxlan interface");
            // Assign subnet IP to interface (first IP in subnet)
            let team_gateway_ip = Ipv4Addr::from(u32::from(*team_subnet) + 1);
            std::process::Command::new("ip")
                .args(["addr", "add", &format!("{}/24", team_gateway_ip), "dev", &vxlan_name])
                .status()
                .expect("Failed to assign IP to vxlan interface");
            // SNAT only traffic from this team's VXLAN interface, using MGMT /24 as --to-source
            let mgmt_cidr = format!("{}/24", mgmt_subnet);
            let team_snat_rule = format!("-s {}/24 -i {} -j SNAT --to-source {}", team_subnet, vxlan_name, mgmt_cidr);
            ipt.append("nat", "POSTROUTING", &team_snat_rule)
                .expect("Failed to add SNAT rule for team");
        }
        std::process::Command::new(ipt.cmd.split_whitespace().next().unwrap())
        .args(ipt.cmd.split_whitespace().skip(1))
        .status()
        .expect("Failed to execute iptables command");
        println!("Competition {}: subnets, SNAT, and VXLAN interfaces set up", competition.name);

    }
    // Execute the firewall rules
    
    // Start Actix-web server for health check
    std::thread::spawn(|| {
        let sys = actix_rt::System::new();
        sys.block_on(async {
            HttpServer::new(|| {
                App::new().service(health)
            })
            .bind(("0.0.0.0", 8000)).expect("Failed to bind Actix server")
            .run()
            .await
            .ok();
        });
    });
    // Keep the main thread alive to continue running the competition setup
    std::thread::park();
}