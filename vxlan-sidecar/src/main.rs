use actix_web::{get, App, HttpServer, Responder};
use carve::config::AppConfig;
use std::env;
use std::process::Command;

#[get("/api/health")]
async fn health() -> impl Responder {
    "Healthy"
}

fn create_vxlan_interface(vxlan_id: &str, cidr: &str, remote: &str) -> Result<(), String> {
    // Remove vxlan0 if it exists
    let _ = Command::new("ip")
        .args(["link", "del", "vxlan0"])
        .status();
    // Create vxlan0 with remote
    let status = Command::new("ip")
        .args(["link", "add", "vxlan0", "type", "vxlan", "id", vxlan_id, "dev", "eth0", "remote", remote, "dstport", "4789"])
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
    // Assign IP
    Command::new("ip")
        .args(["addr", "add", &format!("{}/24", cidr), "dev", "vxlan0"])
        .status()
        .map_err(|e| format!("Failed to assign IP to vxlan0: {}", e))?;
    Ok(())
}

fn create_bridge() -> Result<(), String> {
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
    Ok(())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let vxlan_id = env::var("VXLAN_ID").expect("VXLAN_ID env var required");
    let cidr = env::var("CIDR").expect("CIDR env var required");
    let config = AppConfig::new().expect("Failed to load config");
    // Use vtep_host from the first competition, fallback to localhost if missing
    let remote = config.competitions.get(0)
        .and_then(|c| c.vtep_host.as_deref())
        .unwrap_or("127.0.0.1");
    if let Err(e) = create_vxlan_interface(&vxlan_id, &cidr, remote) {
        eprintln!("{}", e);
    }
    if let Err(e) = create_bridge() {
        eprintln!("{}", e);
    }
    HttpServer::new(|| {
        App::new().service(health)
    })
    .bind(("0.0.0.0", 8000))?
    .run()
    .await
}
