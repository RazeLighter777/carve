use std::env;
use std::fs;
use std::path::Path;
use std::process::{Command};
use anyhow::{anyhow, Context, Result};
use actix_web::{App, HttpServer, get, Responder, HttpResponse};
use carve::config::AppConfig;
use carve::redis_manager::RedisManager;
use ssh_key::{Algorithm, PrivateKey, rand_core::OsRng};
use rand::{Rng};
use actix_web::middleware::Logger;

// Cloud-init file contents will be stored in these variables
#[derive(Clone)]
struct CloudInit {
    user_data: String,
    meta_data: String,
    vendor_data: String,
    network_config: String,
}


#[get("/api/health")]
async fn health_check() -> impl Responder {
    //check if the qemu process is running
    let qemu_pid = fs::read_to_string("/tmp/qemu.pid").ok();
    // if the process is running, return 200 OK
    if let Some(pid) = qemu_pid {
        if let Ok(_) = Command::new("kill").arg("-0").arg(pid.trim()).output() {
            return HttpResponse::Ok().body("QEMU is running");
        }
    }
    HttpResponse::InternalServerError().body("QEMU is not running")
}

#[actix_web::main]
async fn main() -> Result<()> {
    std::env::set_var("RUST_LOG", "debug");

    env_logger::init();
    // Parse environment variables
    let competition = env::var("COMPETITION_NAME").context("COMPETITION_NAME not set")?;
    let box_name = env::var("BOX_NAME").context("BOX_NAME not set")?;
    let team_name = env::var("TEAM_NAME").context("TEAM_NAME not set")?;
    println!("Starting qemu-box for competition: {}, box: {}, team: {}", competition, box_name, team_name);

    // Check config file
    let config_file = "/config/competition.yaml";
    if !Path::new(config_file).exists() {
        return Err(anyhow!("Configuration file not found at {}", config_file));
    }
    // Load competition config
    let app_config = AppConfig::new()?;
    let competition_cfg = app_config.competitions.iter().find(|c| c.name == competition)
        .ok_or_else(|| anyhow!("Competition '{}' not found in config", competition))?;

    // Find first qcow2 image in /disk
    let disk_image = fs::read_dir("/disk")?
        .filter_map(|e| e.ok())
        .find(|e| e.path().extension().map(|x| x == "qcow2").unwrap_or(false))
        .map(|e| e.path())
        .ok_or_else(|| anyhow!("No .qcow2 disk image found in /disk directory"))?;
    let tmp_disk = "/tmp/disk.qcow2";
    fs::copy(&disk_image, tmp_disk)?;
    println!("Using disk image: {}", tmp_disk);

    // Generate cloud-init file contents as variables
    let meta_data_str = format!(r#"instance-id: {box_name}
local-hostname: {box_name}
"#);
    let vendor_data_str = r#"#cloud-config
"#.to_string();
    let mac_address = {
        use rand::Rng;
        let mut rng = rand::rng();
        format!("52:54:00:{:02x}:{:02x}:{:02x}", rng.random::<u8>(), rng.random::<u8>(), rng.random::<u8>())
    };
    println!("Generated MAC address: {}", mac_address);
    let network_config_str = format!(r#"#cloud-config
version: 2
ethernets:
  eth0:
    dhcp4: true
    match:
      macaddress: {mac_address}
"#);
        // --- RedisManager and credentials/keys logic ---
    let redis_mgr = RedisManager::new(&competition_cfg.redis)?;
    // SSH keypair
    let (private_ssh_key,public_ssh_key) = match redis_mgr.read_ssh_keypair(&competition, &team_name, &box_name)? {
        Some(key) => (key.clone(), PrivateKey::from_openssh(&key)?.public_key().to_openssh()?),
        None => {
            let privatekey = PrivateKey::random(&mut OsRng, Algorithm::Ed25519)?;
            let publickey = privatekey.public_key();
            (privatekey.to_openssh(ssh_key::LineEnding::default())?.to_string(), publickey.to_openssh()?)
        }
    };
    // print ssh keypair
    println!("SSH Private Key:\n{}", private_ssh_key);
    println!("SSH Public Key:\n{}", public_ssh_key);
    // Username/password
    let (username, password) = match redis_mgr.read_box_credentials(&competition, &team_name, &box_name)? {
        Some((u, p)) => (u, p),
        None => {
            let username = team_name.clone();
            let password: String = rand::rng()
                .sample_iter(&rand::distr::Alphabetic)
                .take(8)
                .map(char::from)
                .collect();
            let _ = redis_mgr.write_box_credentials(&competition, &team_name, &box_name, &username, &password)?;
            (username, password)
        }
    };
    //print username/password
    println!("Username: {}, Password: {}", username, password);
    // use mkpasswd to hash the password
    let password_hash_stdout = Command::new("mkpasswd")
        .arg("--method=SHA-512")
        .arg("--rounds=4096")
        .arg(&password)
        .output()?
        .stdout;
    let password_hash = String::from_utf8_lossy(&password_hash_stdout);    
    let user_data_str = format!(r#"#cloud-config
users:
  - default
  - name: {username}
    shell: /bin/ash
    lock_passwd: false
    hashed_passwd: {password_hash}
    ssh_authorized_keys:
      - {pubkey}
"#, username=username, password_hash=password_hash, pubkey=public_ssh_key.trim().lines().last().unwrap_or(""));
    
    let cloud_init = CloudInit {
        meta_data : meta_data_str,
        user_data : user_data_str,
        vendor_data : vendor_data_str,
        network_config : network_config_str,
    };
    // Get container IP
    let output = Command::new("hostname").arg("-I").output()?;
    let container_ip = String::from_utf8_lossy(&output.stdout).split_whitespace().next().unwrap_or("").to_string();
    println!("Container IP: {}", container_ip);

    // Generate /etc/qemu/bridge.conf
    let bridge_conf = "/etc/qemu/bridge.conf";
    if !Path::new(bridge_conf).exists() {
        fs::create_dir_all("/etc/qemu")?;
        fs::write(bridge_conf, "allow br0\n")?;
    }
    println!("Using QEMU bridge configuration at {}", bridge_conf);

    // Configure iptables
    let _ = Command::new("iptables")
        .args(["-A", "FORWARD", "-i", "br0", "-m", "physdev", "--physdev-is-bridged", "-j", "ACCEPT"])
        .status();
    println!("Configured iptables to allow traffic from the bridge");

    // Load config and get box resources
    let app_config = AppConfig::new()?;
    let competition_cfg = app_config.competitions.iter().find(|c| c.name == competition)
        .ok_or_else(|| anyhow!("Competition '{}' not found in config", competition))?;
    let box_cfg = competition_cfg.boxes.iter().find(|b| b.name == box_name)
        .ok_or_else(|| anyhow!("Box '{}' not found in competition config", box_name))?;
    let cores = box_cfg.cores.unwrap_or(2); // Default to 2 if not set
    let ram_mb = box_cfg.ram_mb.unwrap_or(1024); // Default to 1024MB if not set
    println!("Box resources: {} cores, {} MB RAM", cores, ram_mb);
    // use cloud-localds to create cloud-init ISO
    let cloud_init_iso = "/tmp/cloud-init.iso";
    let user_data_file = "/tmp/user-data";
    let meta_data_file = "/tmp/meta-data";
    let vendor_data_file = "/tmp/vendor-data";
    let network_config_file = "/tmp/network-config";
    fs::write(user_data_file, &cloud_init.user_data)?;
    fs::write(meta_data_file, &cloud_init.meta_data)?;
    fs::write(vendor_data_file, &cloud_init.vendor_data)?;
    fs::write(network_config_file, &cloud_init.network_config)?;
    let status = Command::new("cloud-localds")
        .args([cloud_init_iso, user_data_file, meta_data_file])
        .status()?;
    if !status.success() {
        return Err(anyhow!("Failed to create cloud-init ISO"));
    }
    // Start QEMU VM
    println!("Starting QEMU VM...");
    let qemu_child = Command::new("qemu-system-x86_64")
        .args([
            "-enable-kvm",
            "-m", &ram_mb.to_string(),
            "-cpu", "host",
            "-smp", &cores.to_string(),
            "-drive", &format!("file={},format=qcow2", tmp_disk),
            "-drive", &format!("file={},index=1,media=cdrom", cloud_init_iso),
            "-net", &format!("nic,model=virtio,macaddr={}", mac_address),
            "-net", "bridge,br=br0",
            "-display", "vnc=0.0.0.0:0,websocket=5700",
            "-daemonize", "-pidfile", "/tmp/qemu.pid"
        ])
        .status()?;
    if !qemu_child.success() {
        return Err(anyhow!("Failed to start QEMU VM"));
    }
    let qemu_pid_val = fs::read_to_string("/tmp/qemu.pid")?.trim().parse::<i32>()?;
    println!("QEMU started with PID: {}", qemu_pid_val);

    // Start actix-web server for cloud-init
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(actix_web::web::Data::new(cloud_init.clone()))
            .service(health_check)
    }).bind(("0.0.0.0", 8001))?
    .run()
    .await?;
    Ok(())
}
