use anyhow::{anyhow, Result};
use carve::redis_manager::RedisManager;
use rand::Rng;
use ssh_key::{rand_core::OsRng, Algorithm, PrivateKey};
use std::fs;
use std::process::Command;

#[derive(Clone)]
pub struct CloudInit {
    pub user_data: String,
    pub meta_data: String,
    pub vendor_data: String,
    pub network_config: String,
}

impl CloudInit {
    pub fn generate_default(
        box_name: &str,
        competition: &str,
        team_name: &str,
        redis_mgr: &RedisManager,
    ) -> Result<(Self, String, String, String)> {
        // meta-data
        let meta_data_str = format!(
            r#"instance-id: {box_name}
local-hostname: {box_name}
"#
        );
        // vendor-data
        let vendor_data_str = r#"#cloud-config
"#
        .to_string();
        // mac address
        let mac_address = format!(
            "52:54:00:{:02x}:{:02x}:{:02x}",
            rand::random::<u8>(),
            rand::random::<u8>(),
            rand::random::<u8>()
        );
        // network-config
        let network_config_str = format!(
            r#"#cloud-config
version: 2
ethernets:
  eth0:
    mtu: 1400
    dhcp4: true
    match:
      macaddress: {mac_address}
"#
        );
        // SSH keypair
        let (private_ssh_key, public_ssh_key) =
            match redis_mgr.read_ssh_keypair(competition, team_name, box_name)? {
                Some(key) => (
                    key.clone(),
                    PrivateKey::from_openssh(&key)?.public_key().to_openssh()?,
                ),
                None => {
                    let privatekey = PrivateKey::random(&mut OsRng, Algorithm::Ed25519)?;
                    let publickey = privatekey.public_key();
                    (
                        privatekey
                            .to_openssh(ssh_key::LineEnding::default())?
                            .to_string(),
                        publickey.to_openssh()?,
                    )
                }
            };
        // password
        // Username/password
        let (username, password) =
            match redis_mgr.read_box_credentials(competition, team_name, box_name)? {
                Some((u, p)) => (u, p),
                None => {
                    let username = team_name;
                    let password: String = rand::rng()
                        .sample_iter(&rand::distr::Alphabetic)
                        .take(8)
                        .map(char::from)
                        .collect();
                    let _ = redis_mgr.write_box_credentials(
                        competition,
                        team_name,
                        box_name,
                        username,
                        &password,
                    )?;
                    (username.to_owned(), password)
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

        let user_data_str = format!(
            r#"#cloud-config
ssh_pwauth: True
package_update: true
package_upgrade: true
runcmd:
    - [ systemctl, enable, --now, ssh ]
users:
  - name: {username}
    shell: /bin/bash
    lock_passwd: false
    hashed_passwd: {password_hash}
    sudo: "ALL=(ALL) NOPASSWD:ALL"
    groups: sudo
    ssh_authorized_keys:
      - {pubkey}
"#,
            username = team_name,
            pubkey = public_ssh_key.trim().lines().last().unwrap_or(""),
            password_hash = password_hash.trim()
        );
        Ok((
            CloudInit {
                meta_data: meta_data_str,
                user_data: user_data_str,
                vendor_data: vendor_data_str,
                network_config: network_config_str,
            },
            mac_address,
            private_ssh_key,
            public_ssh_key,
        ))
    }
}

/// Writes the cloud-init files to disk and creates the ISO using cloud-localds.
/// Returns the path to the created ISO.
pub fn create_cloud_init_files(cloud_init: &CloudInit) -> Result<String> {
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
        .args([cloud_init_iso, user_data_file, meta_data_file, "--network-config", network_config_file])
        .status()?;
    if !status.success() {
        return Err(anyhow!("Failed to create cloud-init ISO"));
    }
    Ok(cloud_init_iso.to_string())
}
