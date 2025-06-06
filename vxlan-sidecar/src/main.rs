use rocket::{get, routes};
use std::env;
use std::process::Command;

#[get("/health")]
fn health() -> &'static str {
    "Healthy"
}

fn create_vxlan_interface(vxlan_id: &str, cidr: &str) -> Result<(), String> {
    // Remove vxlan0 if it exists
    let _ = Command::new("ip")
        .args(["link", "del", "vxlan0"])
        .status();
    // Create vxlan0
    let status = Command::new("ip")
        .args(["link", "add", "vxlan0", "type", "vxlan", "id", vxlan_id, "dev", "eth0", "dstport", "4789"]) // assumes eth0
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

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    let vxlan_id = env::var("VXLAN_ID").expect("VXLAN_ID env var required");
    let cidr = env::var("CIDR").expect("CIDR env var required");
    if let Err(e) = create_vxlan_interface(&vxlan_id, &cidr) {
        eprintln!("{}", e);
    }
    if let Err(e) = create_bridge() {
        eprintln!("{}", e);
    }
    rocket::build().mount("/api", routes![health]).launch()
}
