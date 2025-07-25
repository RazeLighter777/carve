use actix_web::{App, HttpServer, Responder, get};
use carve::{config::AppConfig, redis_manager::{RedisManager, CompetitionStatus}};
use redis::Commands;
use std::collections::HashMap;
use std::net::{Ipv4Addr, ToSocketAddrs};
use std::process::Command;
use std::time::Duration;
use anyhow::{Result, Context, bail};

#[get("/health")]
async fn health() -> impl Responder {
    "Healthy"
}

#[derive(Debug)]
struct NetworkConfig {
    mgmt_subnet: Ipv4Addr,
    team_subnets: Vec<Ipv4Addr>,
}

impl NetworkConfig {
    fn new(cidr: &str, num_teams: usize) -> Result<Self> {
        let (base_ip, prefix) = Self::parse_cidr(cidr)?;
        let mut subnets = Self::allocate_subnets(base_ip, prefix, num_teams + 1)?;
        let mgmt_subnet = subnets.remove(0);
        
        Ok(Self {
            mgmt_subnet,
            team_subnets: subnets,
        })
    }

    fn parse_cidr(cidr: &str) -> Result<(Ipv4Addr, u8)> {
        let parts: Vec<&str> = cidr.split('/').collect();
        if parts.len() != 2 {
            bail!("Invalid CIDR format: {}", cidr);
        }
        
        let ip = parts[0].parse().context("Invalid IP in CIDR")?;
        let prefix = parts[1].parse().context("Invalid prefix in CIDR")?;
        Ok((ip, prefix))
    }

    fn allocate_subnets(base: Ipv4Addr, prefix: u8, num: usize) -> Result<Vec<Ipv4Addr>> {
        let step = 1 << (32 - (prefix + 8)); // /24s from /16
        let mut current = u32::from(base);
        
        let subnets = (0..num)
            .map(|_| {
                let subnet = Ipv4Addr::from(current);
                current += step;
                subnet
            })
            .collect::<Vec<_>>();
        Ok(subnets)
    }
}

struct NetworkManager {
    ipt: iptables::IPTables,
}

impl NetworkManager {
    fn new() -> Result<Self> {
        let ipt = iptables::new(false).expect("Failed to create iptables instance");
        Ok(Self { ipt })
    }

    fn create_vxlan_interface(&self, name: &str, vxlan_id: u32) -> Result<()> {
        // Remove existing interface if it exists
        let _ = Command::new("ip")
            .args(["link", "del", name])
            .status();
        // get ip address of eth0
        let eth0_ip = Command::new("ip")
            .args(["-4", "addr", "show", "dev", "eth0"])
            .output()
            .context("Failed to get eth0 IP address")?;
        if !eth0_ip.status.success() {
            bail!("Failed to get eth0 IP address");
        }
        let eth0_ip = String::from_utf8(eth0_ip.stdout)
            .context("Failed to convert eth0 IP address to string")?;
        let eth0_ip = eth0_ip.lines()
            .find(|line| line.contains("inet "))
            .and_then(|line| line.split_whitespace().nth(1))
            .context("Failed to parse eth0 IP address")?;
        let eth0_ip = eth0_ip.split('/').next().context("Failed to split eth0 IP address")?;
        println!("Using eth0 IP address: {}", eth0_ip);
        let status = Command::new("ip")
            .args([
                "link", "add", name, "type", "vxlan", "id", &vxlan_id.to_string(),  "nolearning", "dstport", "4789",
            ])
            .status()
            .context("Failed to create VXLAN interface")?;

        if !status.success() {
            bail!("Failed to create VXLAN interface {}", name);
        }

        Command::new("ip")
            .args(["link", "set", name, "up"])
            .status()
            .context("Failed to bring up VXLAN interface")?;
        // Set MTU to 1370
        Command::new("ip")
            .args(["link", "set", name, "mtu", "1370"])
            .status()
            .context("Failed to set MTU for VXLAN interface")?;
        Ok(())
    }

    fn create_bridge_with_vxlan(&self, bridge_name: &str, vxlan_name: &str, gateway_ip: Ipv4Addr) -> Result<()> {
        // Create bridge
        let status = Command::new("ip")
            .args(["link", "add", bridge_name, "type", "bridge"])
            .status()
            .context("Failed to create bridge interface")?;

        if !status.success() {
            bail!("Failed to create bridge interface {}", bridge_name);
        }

        // Add VXLAN to bridge
        Command::new("ip")
            .args(["link", "set", vxlan_name, "master", bridge_name])
            .status()
            .context("Failed to add VXLAN interface to bridge")?;

        // Bring up bridge
        Command::new("ip")
            .args(["link", "set", bridge_name, "up"])
            .status()
            .context("Failed to bring up bridge interface")?;

        // set bridge MTU
        Command::new("ip")
            .args(["link", "set", bridge_name, "mtu", "1370"])
            .status()
            .context("Failed to set MTU for bridge interface")?;

        // Assign IP to bridge
        Command::new("ip")
            .args(["addr", "add", &format!("{}/24", gateway_ip), "dev", bridge_name])
            .status()
            .context("Failed to assign IP to bridge interface")?;

        Ok(())
    }

    fn setup_team_rules(&self, bridge_name: &str) -> Result<()> {
        // SNAT rule
        let snat_rule = format!("-o {} -j MASQUERADE", bridge_name);
        self.ipt.append("nat", "POSTROUTING", &snat_rule)
            .expect("Failed to add SNAT rule");

        // TTL rule
        let ttl_rule = format!("-i {} -j TTL --ttl-set 64", bridge_name);
        self.ipt.append("mangle", "PREROUTING", &ttl_rule)
            .expect("Failed to add TTL rule");

        Ok(())
    }

    fn manage_nat_rule(&self, enable: bool, rule_active: &mut bool) -> Result<()> {
        let nat_rule = "-o eth0 -j MASQUERADE";
        
        match (enable, *rule_active) {
            (true, false) => {
                self.ipt.append("nat", "POSTROUTING", nat_rule)
                    .expect("Failed to add NAT rule");
                *rule_active = true;
                println!("NAT enabled for outgoing traffic");
            }
            (false, true) => {
                self.ipt.delete("nat", "POSTROUTING", nat_rule)
                    .expect("Failed to remove NAT rule");
                *rule_active = false;
                println!("NAT disabled for outgoing traffic");
            }
            _ => {} // No change needed
        }
        Ok(())
    }
}

struct FdbManager;

impl FdbManager {
    async fn update_fdb_entries(config: &AppConfig) -> Result<()> {
        for (comp_idx, competition) in config.competitions.iter().enumerate() {
            let redis_manager = RedisManager::new(&competition.redis)
                .context("Failed to create Redis manager")?;

            for (team_idx, team) in competition.teams.iter().enumerate() {
                let vxlan_name = format!("vxlan_{}_{}", comp_idx, team_idx);
                let mac_address = Self::get_interface_mac(&vxlan_name)?;
                
                Self::publish_fdb_entry(&redis_manager, competition, &mac_address, team).await?;
                Self::update_bridge_fdb(&redis_manager, competition, team, &vxlan_name, &mac_address).await?;
            }
        }
        Ok(())
    }

    fn get_interface_mac(interface: &str) -> Result<String> {
        let output = Command::new("cat")
            .arg(format!("/sys/class/net/{}/address", interface))
            .output()
            .context("Failed to get MAC address")?;

        String::from_utf8(output.stdout)
            .context("Failed to convert MAC address to string")
            .map(|s| s.trim().to_string())
    }

    async fn publish_fdb_entry(
        redis_manager: &RedisManager,
        competition: &carve::config::Competition,
        mac_address: &str,
        team: &carve::config::Team,
    ) -> Result<()> {
        let host = format!("{}:4789", competition.vtep_host.as_ref().context("vtep_host missing")?);
        let addr = host.to_socket_addrs()
            .context("Failed to resolve host")?
            .next()
            .context("No addresses found")?;

        redis_manager.create_vxlan_fdb_entry(&competition.name, mac_address, addr.ip(), &team.name)
            .await
            .context("Failed to create VXLAN FDB entry")?;

        println!("Published FDB entry for {}: {} -> {}", team.name, addr.ip(), mac_address);
        Ok(())
    }

    async fn update_bridge_fdb(
        redis_manager: &RedisManager,
        competition: &carve::config::Competition,
        team: &carve::config::Team,
        vxlan_name: &str,
        our_mac: &str,
    ) -> Result<()> {
        let fdb_entries = redis_manager.get_domain_fdb_entries(&competition.name, &team.name)
            .await
            .context("Failed to get FDB entries")?;

        for (mac, addr) in fdb_entries {
            if mac == our_mac {
                println!("Skipping our own MAC address: {}", mac);
                continue; // Skip our own MAC
            }
            Self::add_fdb_entry(vxlan_name, &mac, &addr, false)?;
            Self::add_fdb_entry(vxlan_name, "00:00:00:00:00:00", &addr, true)?;
        }
        Ok(())
    }

    fn add_fdb_entry(vxlan_name: &str, mac: &str, addr: &str, is_broadcast: bool) -> Result<()> {
        let status = Command::new("bridge")
            .args(["fdb", "append", mac, "dev", vxlan_name, "dst", addr])
            .status()
            .context("Failed to add FDB entry")?;

        if !status.success() {
            bail!("Failed to add {} FDB entry: {}", 
                if is_broadcast { "broadcast" } else { "unicast" }, status);
        }

        println!("Added {} FDB entry: {} -> {}", 
            if is_broadcast { "broadcast" } else { "unicast" }, mac, addr);
        Ok(())
    }
}

fn setup_competition_network(competition: &carve::config::Competition, comp_idx: usize) -> Result<()> {
    let cidr = competition.cidr.as_ref().context("competition.cidr missing")?;
    let network_config = NetworkConfig::new(cidr, competition.teams.len())?;
    let network_manager = NetworkManager::new()?;

    // Setup Redis
    let redis_url = format!("redis://{}:{}/{}", 
        competition.redis.host, competition.redis.port, competition.redis.db);
    let client = redis::Client::open(redis_url).context("Failed to create Redis client")?;
    let mut con = client.get_connection().context("Failed to get Redis connection")?;

    // Clean and populate subnet data
    let _: () = redis::cmd("DEL")
        .arg(format!("{}:subnets", competition.name))
        .query(&mut con)
        .context("Failed to clean subnets hash")?;

    let mut subnet_map = HashMap::new();
    subnet_map.insert("MGMT".to_string(), format!("{}/24,MGMT,0", network_config.mgmt_subnet));

    // Setup MGMT VXLAN
    let vxlan_mgmt_name = format!("vxlan_mgmt_{}", comp_idx);
    network_manager.create_vxlan_interface(&vxlan_mgmt_name, 1337)?;
    
    let mgmt_gateway_ip = Ipv4Addr::from(u32::from(network_config.mgmt_subnet) + 1);
    Command::new("ip")
        .args(["addr", "add", &format!("{}/24", mgmt_gateway_ip), "dev", &vxlan_mgmt_name])
        .status()
        .context("Failed to assign IP to MGMT VXLAN interface")?;

    // Setup team networks
    for (i, (team, &subnet)) in competition.teams.iter().zip(&network_config.team_subnets).enumerate() {
        let vxlan_id = 1338 + i as u32;
        let vxlan_name = format!("vxlan_{}_{}", comp_idx, i);
        let bridge_name = format!("br_{}_{}", comp_idx, i);

        println!("Creating VXLAN interface for {} named {}", team.name, vxlan_name);

        // Create VXLAN interface
        network_manager.create_vxlan_interface(&vxlan_name, vxlan_id)?;

        // Create bridge and setup
        let team_gateway_ip = Ipv4Addr::from(u32::from(subnet) + 1);
        network_manager.create_bridge_with_vxlan(&bridge_name, &vxlan_name, team_gateway_ip)?;

        // Setup iptables rules
        network_manager.setup_team_rules(&bridge_name)?;

        // Add to subnet map
        subnet_map.insert(team.name.clone(), format!("{}/24,{},{}", subnet, team.name, vxlan_id));
    }

    // Store subnet map in Redis
    let subnet_pairs: Vec<_> = subnet_map.iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    
    let _: () = con.hset_multiple(format!("{}:subnets", competition.name), &subnet_pairs)
        .context("Failed to store subnet map in Redis")?;

    Ok(())
}

fn start_web_server() {
    tokio::spawn(async move {
        let sys = actix_rt::System::new();
        sys.block_on(async {
            if let Err(e) = HttpServer::new(|| App::new().service(health))
                .bind(("0.0.0.0", 8000))
                .context("Failed to bind Actix server")
                .and_then(|server| Ok(server.run()))
                .unwrap()
                .await
            {
                eprintln!("Web server error: {}", e);
            }
        });
    });
}

fn start_fdb_update_thread(config: AppConfig) {
    tokio::spawn(async move {
        loop {
            if let Err(e) = FdbManager::update_fdb_entries(&config).await {
                eprintln!("FDB update error: {}", e);
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });
}

async fn manage_competition_nat(config: &AppConfig) -> Result<()> {
    let redis_manager = RedisManager::new(&config.competitions[0].redis)
        .context("Failed to create Redis manager")?;
    let network_manager = NetworkManager::new()?;
    let mut rule_added = false;

    // Handle initial state
    let competition_name = &config.competitions[0].name;
    let current_state = redis_manager.get_competition_state(competition_name)
        .await
        .context("Failed to get competition state")?;

    let should_enable_nat = matches!(current_state.status, CompetitionStatus::Unstarted);
    network_manager.manage_nat_rule(should_enable_nat, &mut rule_added)?;

    // Handle state changes
    loop {
        match redis_manager.wait_for_competition_event(competition_name) {
            Ok(event) => {
                let should_enable_nat = matches!(event.status, CompetitionStatus::Unstarted);
                if let Err(e) = network_manager.manage_nat_rule(should_enable_nat, &mut rule_added) {
                    eprintln!("Failed to manage NAT rule: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Error waiting for competition event: {}", e);
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = AppConfig::new().context("Failed to load config")?;

    // Setup network for each competition
    for (comp_idx, competition) in config.competitions.iter().enumerate() {
        setup_competition_network(competition, comp_idx)
            .with_context(|| format!("Failed to setup network for competition {}", competition.name))?;
    }

    // Start background services
    start_web_server();
    start_fdb_update_thread(config.clone());

    // Manage NAT rules based on competition state
    manage_competition_nat(&config).await?;

    Ok(())
}