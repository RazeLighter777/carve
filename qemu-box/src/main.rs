use actix_web::middleware::Logger;
use actix_web::{get, App, HttpResponse, HttpServer, Responder};
use anyhow::{anyhow, Context, Result};
use carve::{
    config::AppConfig,
    redis_manager::{QemuCommands, RedisManager},
};
use std::{
    env, fs,
    io::{Read, Write},
    net::ToSocketAddrs,
    path::Path,
    process::Command,
    thread,
    time::Duration,
};

mod cloud_init;
use cloud_init::{create_cloud_init_files, CloudInit};

// Environment configuration struct
#[derive(Debug, Clone)]
struct EnvConfig {
    competition: String,
    box_name: String,
    team_name: String,
}

impl EnvConfig {
    fn from_env() -> Result<Self> {
        Ok(Self {
            competition: env::var("COMPETITION_NAME").context("COMPETITION_NAME not set")?,
            box_name: env::var("BOX_NAME").context("BOX_NAME not set")?,
            team_name: env::var("TEAM_NAME").context("TEAM_NAME not set")?,
        })
    }
}

// QEMU monitor interface
struct QemuMonitor;

impl QemuMonitor {
    const SOCKET_PATH: &'static str = "/run/qemu-monitor.sock";

    fn read_response(stream: &mut std::os::unix::net::UnixStream) -> Result<String> {
        let mut buffer = [0; 512];
        let mut response = Vec::new();

        loop {
            let bytes_read = stream.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            response.extend_from_slice(&buffer[..bytes_read]);

            if response.ends_with(b"(qemu) ") {
                break;
            }
        }

        Ok(std::str::from_utf8(&response)?.to_owned())
    }

    fn send_command(command: &str) -> Result<String> {
        use std::os::unix::net::UnixStream;

        let mut stream = UnixStream::connect(Self::SOCKET_PATH)
            .context("Failed to connect to QEMU monitor socket")?;

        // Clear initial prompt
        Self::read_response(&mut stream)?;

        // Send command
        stream.write_all(command.as_bytes())?;
        stream.flush()?;

        // Read response
        Self::read_response(&mut stream)
    }

    fn snapshot(team_name: &str, box_name: &str) -> Result<()> {
        let command = format!("savevm {}_{}\n", team_name, box_name);
        let response = Self::send_command(&command)?;
        println!("Snapshot command sent. Response: {}", response);
        Ok(())
    }

    fn restore(team_name: &str, box_name: &str) -> Result<()> {
        let command = format!("loadvm {}_{}\n", team_name, box_name);
        let response = Self::send_command(&command)?;
        println!("Restore command sent. Response: {}", response);
        Ok(())
    }
}

// VM Manager to handle QEMU operations
struct VmManager {
    env_config: EnvConfig,
    app_config: AppConfig,
}

impl VmManager {
    fn new(env_config: EnvConfig) -> Result<Self> {
        let app_config = AppConfig::new()?;
        Ok(Self {
            env_config,
            app_config,
        })
    }

    fn get_competition_config(&self) -> Result<&carve::config::Competition> {
        self.app_config
            .competitions
            .iter()
            .find(|c| c.name == self.env_config.competition)
            .ok_or_else(|| anyhow!("Competition '{}' not found", self.env_config.competition))
    }

    fn get_box_config(&self) -> Result<&carve::config::Box> {
        let competition_cfg = self.get_competition_config()?;
        competition_cfg
            .boxes
            .iter()
            .find(|b| b.name == self.env_config.box_name)
            .ok_or_else(|| anyhow!("Box '{}' not found", self.env_config.box_name))
    }

    fn prepare_disk_image(&self) -> Result<String> {
        let box_cfg = self.get_box_config()?;
        let disk_image = &box_cfg.backing_image;
        let tmp_disk = "/tmp/disk.img";

        println!("Using disk image: {}", disk_image);

        let format = if disk_image.starts_with("nbd://") {
            "raw"
        } else {
            "qcow2"
        };

        let status = Command::new("qemu-img")
            .args([
                "create", "-f", "qcow2", "-F", format, "-b", disk_image, tmp_disk,
            ])
            .status()?;

        if !status.success() {
            return Err(anyhow!("Failed to create qcow2 image with backing file"));
        }

        println!(
            "Created qcow2 image at {} with backing file {}",
            tmp_disk, disk_image
        );
        Ok(tmp_disk.to_string())
    }

    fn setup_network(&self) -> Result<()> {
        // Create bridge configuration
        let bridge_conf = "/etc/qemu/bridge.conf";
        if !Path::new(bridge_conf).exists() {
            fs::create_dir_all("/etc/qemu")?;
            fs::write(bridge_conf, "allow br0\n")?;
        }

        // Configure iptables (ignore errors as this might already be configured)
        let _ = Command::new("iptables")
            .args([
                "-A",
                "FORWARD",
                "-m",
                "physdev",
                "--physdev-is-bridged",
                "-j",
                "ACCEPT",
            ])
            .status();

        println!("Network configuration complete");
        Ok(())
    }

    fn start_qemu(&self, disk_path: &str, cloud_init_iso: &str, mac_address: &str) -> Result<()> {
        let box_cfg = self.get_box_config()?;
        let cores = box_cfg.cores.unwrap_or(2);
        let ram_mb = box_cfg.ram_mb.unwrap_or(1024);

        println!("Starting QEMU VM with {} cores, {} MB RAM", cores, ram_mb);
        
        let status = Command::new("qemu-system-x86_64")
            .args([
                "-enable-kvm",
                "-m",
                &ram_mb.to_string(),
                "-cpu",
                "host",
                "-smp",
                &cores.to_string(),
                "-drive",
                &format!("file={},format=qcow2", disk_path),
                "-drive",
                &format!("file={},index=1,media=cdrom", cloud_init_iso),
                "-net",
                &format!("nic,model=virtio,macaddr={}", mac_address),
                "-net",
                "bridge,br=br0",
                "-display",
                "vnc=0.0.0.0:0,websocket=5700,power-control=on",
                "-daemonize",
                "-pidfile",
                "/tmp/qemu.pid",
                "-monitor",
                "unix:/run/qemu-monitor.sock,server,nowait",
            ])
            .status()?;

        if !status.success() {
            return Err(anyhow!("Failed to start QEMU VM"));
        }

        let pid = fs::read_to_string("/tmp/qemu.pid")?.trim().parse::<i32>()?;
        println!("QEMU started with PID: {}", pid);

        Ok(())
    }
}

// Background task manager
struct TaskManager {
    env_config: EnvConfig,
    redis_mgr: RedisManager,
    mac_address: String,
}

impl TaskManager {
    fn new(env_config: EnvConfig, redis_mgr: RedisManager, mac_address: &str) -> Self {
        Self {
            env_config,
            redis_mgr,
            mac_address: mac_address.to_string(),
        }
    }

    fn start_qemu_event_listener(&self) {
        let redis_mgr = self.redis_mgr.clone();
        let env_config = self.env_config.clone();

        thread::spawn(move || loop {
            match redis_mgr.wait_for_qemu_event(
                &env_config.competition,
                &env_config.team_name,
                &env_config.box_name,
                vec![QemuCommands::Snapshot, QemuCommands::Restore].into_iter(),
            ) {
                Ok(QemuCommands::Snapshot) => {
                    println!("Received QEMU snapshot command");
                    if let Err(e) =
                        QemuMonitor::snapshot(&env_config.team_name, &env_config.box_name)
                    {
                        eprintln!("Failed to create snapshot: {}", e);
                    }
                }
                Ok(QemuCommands::Restore) => {
                    println!("Received QEMU restore command");
                    if let Err(e) =
                        QemuMonitor::restore(&env_config.team_name, &env_config.box_name)
                    {
                        eprintln!("Failed to restore snapshot: {}", e);
                    }
                }
                _ => eprintln!("Error waiting for QEMU event"),
            }
        });
    }

    fn start_vxlan_updater(&self) {
        let redis_mgr = self.redis_mgr.clone();
        let env_config = self.env_config.clone();
        let mac_address = self.mac_address.clone();

        thread::spawn(move || loop {
            let vxlan_sidecar_addr = format!(
                "vxlan-sidecar-{}-{}:4789",
                env_config.team_name, env_config.box_name
            );

            match vxlan_sidecar_addr.to_socket_addrs() {
                Ok(addrs) => {
                    for addr in addrs {
                        if let Err(e) = redis_mgr.create_vxlan_fdb_entry(
                            &env_config.competition,
                            &mac_address,
                            addr.ip(),
                            &env_config.team_name,
                        ) {
                            eprintln!("Failed to create VXLAN FDB entry: {}", e);
                        } else {
                            println!("Created VXLAN FDB entry: {} -> {}", mac_address, addr.ip());
                        }
                    }
                }
                Err(e) => eprintln!("Failed to resolve vxlan-sidecar service: {}", e),
            }

            thread::sleep(Duration::from_secs(5));
        });
    }
}

#[get("/api/health")]
async fn health_check() -> impl Responder {
    match fs::read_to_string("/tmp/qemu.pid") {
        Ok(pid) => {
            if Command::new("kill")
                .args(["-0", pid.trim()])
                .status()
                .is_ok()
            {
                HttpResponse::Ok().body("QEMU is running")
            } else {
                HttpResponse::InternalServerError().body("QEMU is not running")
            }
        }
        Err(_) => HttpResponse::InternalServerError().body("QEMU is not running"),
    }
}

#[actix_web::main]
async fn main() -> Result<()> {
    env::set_var("RUST_LOG", "debug");
    env_logger::init();

    let env_config = EnvConfig::from_env()?;
    println!(
        "Starting qemu-box for competition: {}, box: {}, team: {}",
        env_config.competition, env_config.box_name, env_config.team_name
    );

    // Validate config file exists
    let config_file = "/config/competition.yaml";
    if !Path::new(config_file).exists() {
        return Err(anyhow!("Configuration file not found at {}", config_file));
    }

    // Initialize VM manager
    let vm_manager = VmManager::new(env_config.clone())?;
    let competition_cfg = vm_manager.get_competition_config()?;

    // Setup Redis connection
    let redis_mgr = RedisManager::new(&competition_cfg.redis)?;

    // Generate cloud-init and networking configuration
    let (cloud_init, mac_address, private_key, public_key) = CloudInit::generate_default(
        &env_config.box_name,
        &env_config.competition,
        &env_config.team_name,
        &redis_mgr,
    )?;

    println!("SSH Private Key:\n{}", private_key);
    println!("SSH Public Key:\n{}", public_key);
    println!("Generated MAC address: {}", mac_address);

    // Prepare VM environment
    let disk_path = vm_manager.prepare_disk_image()?;
    vm_manager.setup_network()?;
    let cloud_init_iso = create_cloud_init_files(&cloud_init)?;

    // Start QEMU

    // Start background tasks
    let task_manager = TaskManager::new(env_config, redis_mgr, &mac_address);
    task_manager.start_vxlan_updater();
    vm_manager.start_qemu(&disk_path, &cloud_init_iso, &mac_address)?;
    task_manager.start_qemu_event_listener();

    // Start HTTP server
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(actix_web::web::Data::new(cloud_init.clone()))
            .service(health_check)
    })
    .bind(("0.0.0.0", 8001))?
    .run()
    .await?;

    Ok(())
}
