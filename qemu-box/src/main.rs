use actix_web::middleware::Logger;
use actix_web::{get, App, HttpResponse, HttpServer, Responder};
use anyhow::{anyhow, Context, Result};
use carve::config::AppConfig;
use carve::redis_manager::RedisManager;
use std::env;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::process::Command;

mod cloud_init;
use cloud_init::{create_cloud_init_files, CloudInit};

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

fn qemu_read(reader: &mut std::os::unix::net::UnixStream) -> Result<String> {
    // @TODO add timeout
    let mut buffer = [0; 512];
    let mut response = Vec::new();
    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break; // End of stream
        }
        response.extend_from_slice(&buffer[..bytes_read]);

        // Check if we have received the QEMU prompt indicating the end of the response
        if response.ends_with(b"(qemu) ") {
            break;
        }
    }

    // Convert the response to a string
    Ok(std::str::from_utf8(&response)?.to_owned())
}

#[actix_web::main]
async fn main() -> Result<()> {
    std::env::set_var("RUST_LOG", "debug");

    env_logger::init();
    // Parse environment variables
    let competition = env::var("COMPETITION_NAME").context("COMPETITION_NAME not set")?;
    let box_name = env::var("BOX_NAME").context("BOX_NAME not set")?;
    let team_name = env::var("TEAM_NAME").context("TEAM_NAME not set")?;
    println!(
        "Starting qemu-box for competition: {}, box: {}, team: {}",
        competition, box_name, team_name
    );

    // Check config file
    let config_file = "/config/competition.yaml";
    if !Path::new(config_file).exists() {
        return Err(anyhow!("Configuration file not found at {}", config_file));
    }
    // Load competition config
    let app_config = AppConfig::new()?;
    let competition_cfg = app_config
        .competitions
        .iter()
        .find(|c| c.name == competition)
        .ok_or_else(|| anyhow!("Competition '{}' not found in config", competition))?;

    // Find first qcow2 image in /disk
    let disk_image = fs::read_dir("/disk")?
        .filter_map(|e| e.ok())
        .find(|e| e.path().extension().map(|x| x == "qcow2").unwrap_or(false))
        .map(|e| e.path())
        .ok_or_else(|| anyhow!("No .qcow2 disk image found in /disk directory"))?;
    let tmp_disk = "/tmp/disk.qcow2";
    // Create a new qcow2 image in /tmp with the original as a backing file
    let status = Command::new("qemu-img")
        .args([
            "create",
            "-f",
            "qcow2",
            "-F",
            "qcow2",
            "-b",
            disk_image.to_str().unwrap(),
            tmp_disk,
        ])
        .status()?;
    if !status.success() {
        return Err(anyhow!("Failed to create qcow2 image with backing file"));
    }
    println!(
        "Created qcow2 image at {} with backing file {}",
        tmp_disk,
        disk_image.display()
    );

    // --- RedisManager and credentials/keys logic ---
    let redis_mgr = RedisManager::new(&competition_cfg.redis)?;
    // Generate cloud-init, mac address, and SSH keys using the module
    let (cloud_init, mac_address, private_ssh_key, public_ssh_key) =
        CloudInit::generate_default(&box_name, &competition, &team_name, &redis_mgr)?;
    // print ssh keypair
    println!("SSH Private Key:\n{}", private_ssh_key);
    println!("SSH Public Key:\n{}", public_ssh_key);
    println!("Generated MAC address: {}", mac_address);
    // Remove user_data_str and direct assignment to cloud_init.user_data
    let cloud_init = CloudInit {
        meta_data: cloud_init.meta_data,
        user_data: cloud_init.user_data,
        vendor_data: cloud_init.vendor_data,
        network_config: cloud_init.network_config,
    };
    // Get container IP
    let output = Command::new("hostname").arg("-I").output()?;
    let container_ip = String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_string();
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
        .args([
            "-A",
            "FORWARD",
            "-i",
            "br0",
            "-m",
            "physdev",
            "--physdev-is-bridged",
            "-j",
            "ACCEPT",
        ])
        .status();
    println!("Configured iptables to allow traffic from the bridge");

    // Load config and get box resources
    let app_config = AppConfig::new()?;
    let competition_cfg = app_config
        .competitions
        .iter()
        .find(|c| c.name == competition)
        .ok_or_else(|| anyhow!("Competition '{}' not found in config", competition))?;
    let box_cfg = competition_cfg
        .boxes
        .iter()
        .find(|b| b.name == box_name)
        .ok_or_else(|| anyhow!("Box '{}' not found in competition config", box_name))?;
    let cores = box_cfg.cores.unwrap_or(2); // Default to 2 if not set
    let ram_mb = box_cfg.ram_mb.unwrap_or(1024); // Default to 1024MB if not set
    println!("Box resources: {} cores, {} MB RAM", cores, ram_mb);
    // use cloud-localds to create cloud-init ISO
    let cloud_init_iso = create_cloud_init_files(&cloud_init)?;
    if let Ok(code) = redis_mgr.get_box_console_code(&competition, &team_name) {
        // Start QEMU VM
        println!("Starting QEMU VM...");
        let set_password_command_going_to_stdin = format!("set_password vnc {}\n", code);
        use std::io::Write;
        use std::process::Stdio;
        let mut qemu_child = Command::new("qemu-system-x86_64")
            .args([
                "-enable-kvm",
                "-m",
                &ram_mb.to_string(),
                "-cpu",
                "host",
                "-smp",
                &cores.to_string(),
                "-drive",
                &format!("file={},format=qcow2", tmp_disk),
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
            .stdin(Stdio::piped())
            // stdout/stderr will go to parent (inherited)
            .spawn()?;
        if let Some(mut stdin) = qemu_child.stdin.take() {
            stdin.write_all(set_password_command_going_to_stdin.as_bytes())?;
        }
        // print stdout and stderr
        if let Some(stdout) = qemu_child.stdout.take() {
            use std::io::{BufRead, BufReader};
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                println!("QEMU stdout: {}", line?);
            }
        }
        let status = qemu_child.wait()?;
        if !status.success() {
            return Err(anyhow!("Failed to start QEMU VM"));
        }
        let qemu_pid_val = fs::read_to_string("/tmp/qemu.pid")?.trim().parse::<i32>()?;
        println!("QEMU started with PID: {}", qemu_pid_val);
        // start new thread to wait for redis subscription for qemu events
        let _ = std::thread::spawn(move || {
            loop {
                match redis_mgr.wait_for_qemu_event(
                    &competition,
                    &team_name,
                    &box_name,
                    vec![
                        carve::redis_manager::QemuCommands::Snapshot,
                        carve::redis_manager::QemuCommands::Restore,
                    ]
                    .into_iter(),
                ) {
                    Ok(event) => {
                        if event == carve::redis_manager::QemuCommands::Snapshot {
                            println!("Received QEMU snapshot command");
                            // Handle restart logic here
                            #[cfg(unix)]
                            {
                                use std::os::unix::net::UnixStream;
                                // Connect to the QEMU monitor socket
                                let monitor_socket = "/run/qemu-monitor.sock";
                                if let Ok(mut stream) = UnixStream::connect(monitor_socket) {
                                    qemu_read(&mut stream).unwrap();
                                    // Send the 'savevm' command to QEMU
                                    let command = format!("savevm {}_{} \n", team_name, box_name);
                                    if let Err(e) = stream.write_all(command.as_bytes()) {
                                        eprintln!("Failed to send command to QEMU monitor: {}", e);
                                    } else {
                                        stream.flush().unwrap();
                                        println!("Sent 'Snapshot' command to QEMU monitor");
                                        //print output
                                        if let Ok(response) = qemu_read(&mut stream) {
                                            println!("QEMU response: {}", response);
                                        } else {
                                            eprintln!("Failed to read response from QEMU monitor");
                                        }
                                    }
                                } else {
                                    eprintln!(
                                        "Failed to connect to QEMU monitor socket at {}",
                                        monitor_socket
                                    );
                                }
                            }
                        } else if event == carve::redis_manager::QemuCommands::Restore {
                            println!("Received QEMU restore command");
                            // Handle restore logic here
                            #[cfg(unix)]
                            {
                                use std::os::unix::net::UnixStream;
                                // Connect to the QEMU monitor socket
                                let monitor_socket = "/run/qemu-monitor.sock";
                                if let Ok(mut stream) = UnixStream::connect(monitor_socket) {
                                    qemu_read(&mut stream).unwrap();
                                    let command = format!("loadvm {}_{} \n", team_name, box_name);
                                    if let Err(e) = stream.write_all(command.as_bytes()) {
                                        eprintln!("Failed to send command to QEMU monitor: {}", e);
                                    } else {
                                        println!("Sent 'Restore' command to QEMU monitor");
                                        // print output
                                        if let Ok(response) = qemu_read(&mut stream) {
                                            println!("QEMU response: {}", response);
                                        } else {
                                            eprintln!("Failed to read response from QEMU monitor");
                                        }
                                    }
                                } else {
                                    eprintln!(
                                        "Failed to connect to QEMU monitor socket at {}",
                                        monitor_socket
                                    );
                                }
                            }
                        }
                        // Handle the event (e.g., shutdown, reset, etc.)
                    }
                    Err(e) => eprintln!("Error waiting for QEMU event: {}", e),
                }
            }
        });
        // Start actix-web server for cloud-init
        HttpServer::new(move || {
            App::new()
                .wrap(Logger::default())
                .app_data(actix_web::web::Data::new(cloud_init.clone()))
                .service(health_check)
        })
        .bind(("0.0.0.0", 8001))?
        .run()
        .await?;
    }
    Ok(())
}
