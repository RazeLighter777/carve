use anyhow::{Context, Result};
use regex::Regex;
use ssh2::Session;
use std::time::Duration;
use std::{net::TcpStream, os::unix::fs::PermissionsExt};

use carve::config::{CheckSpec, HttpCheckSpec, HttpMethods, IcmpCheckSpec, SshCheckSpec};

pub async fn perform_http_check(hostname: &str, spec: &HttpCheckSpec) -> Result<String> {
    let client = reqwest::Client::new();
    let url = format!("http://{}{}", hostname, spec.url);

    let response = client
        .request(
            match spec.method {
                HttpMethods::Get => reqwest::Method::GET,
                HttpMethods::Post => reqwest::Method::POST,
                HttpMethods::Put => reqwest::Method::PUT,
                HttpMethods::Delete => reqwest::Method::DELETE,
            },
            url.clone(),
        )
        .timeout(Duration::from_secs(5))
        .header("Content-Type", if spec.method == HttpMethods::Post {
            "application/x-www-form-urlencoded"
        } else {
            "application/json"
        })
        .body(
            spec.forms
                .as_ref()
                .map(|forms| forms.clone())
                .unwrap_or_default(),
        )
        .send()
        .await
        .context("Failed to send HTTP request")?;

    let status = response.status();
    let body = response
        .text()
        .await
        .context("Failed to get response body")?;

    if status.as_u16() != spec.code {
        return Err(anyhow::anyhow!(
            "HTTP status code mismatch: expected {}, got {}",
            spec.code,
            status.as_u16()
        ));
    }

    let re = Regex::new(&spec.regex).context("Invalid regex pattern")?;
    if !re.is_match(&body) {
        return Err(anyhow::anyhow!(
            "Response body does not match regex: {}",
            spec.regex
        ));
    }

    Ok(format!("HTTP check successful: {}", url))
}

pub fn perform_nix_check(hostname: &str, spec: &carve::config::NixCheckSpec) -> Result<String> {
    //Write the script to a temporary file
    let temp_script_path = std::env::temp_dir().join("nix_check_script.sh");
    std::fs::write(&temp_script_path, &spec.script)
        .context("Failed to write Nix check script to temporary file")?;
    // Make the script executable
    std::fs::set_permissions(&temp_script_path, std::fs::Permissions::from_mode(0o755))
        .context("Failed to set permissions on Nix check script")?;
    // Execute the script
    let output = std::process::Command::new(&temp_script_path)
        .arg(hostname)
        .output()
        .context("Failed to execute Nix check script")?;
    // Clean up the temporary file
    std::fs::remove_file(temp_script_path)
        .context("Failed to remove temporary Nix check script file")?;
    if output.status.success() {
        Ok(format!(
            "Nix check successful: {}",
            String::from_utf8_lossy(&output.stdout)
        ))
    } else {
        Err(anyhow::anyhow!(
            "Nix check failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

pub fn perform_icmp_check(hostname: &str, spec: &IcmpCheckSpec) -> Result<String> {
    // Simplify the ICMP check for now
    // In a real implementation, we would use proper DNS resolution and error handling
    // This is a simplified version that just checks if the host responds to ping

    // Use hostname directly (the ping library will handle resolution)
    let result = std::process::Command::new("ping")
        .args(["-c", "1", "-W", "5", hostname])
        .output()
        .context("Failed to execute ping command")?;

    let success = result.status.success();

    // Check if the result matches our expectation
    if (success && spec.code == 0) || (!success && spec.code != 0) {
        Ok(format!("ICMP check successful: {}", hostname))
    } else {
        Err(anyhow::anyhow!(
            "ICMP check failed: expected code {}, got {}",
            spec.code,
            if success { 0 } else { 1 }
        ))
    }
}

pub fn perform_ssh_check(hostname: &str, spec: &SshCheckSpec) -> Result<String> {
    let tcp = TcpStream::connect(format!("{}:{}", hostname, spec.port))
        .context("Failed to connect to SSH server")?;

    let mut session = Session::new().context("Failed to create SSH session")?;
    session.set_tcp_stream(tcp);
    session.handshake().context("Failed SSH handshake")?;

    if let Some(password) = &spec.password {
        session
            .userauth_password(&spec.username, password)
            .context("SSH authentication failed")?;
    } else if let Some(key_path) = &spec.key_path {
        session
            .userauth_pubkey_file(&spec.username, None, std::path::Path::new(key_path), None)
            .context("SSH key authentication failed")?;
    } else {
        return Err(anyhow::anyhow!("No SSH authentication method provided"));
    }

    if !session.authenticated() {
        return Err(anyhow::anyhow!("SSH authentication failed"));
    }

    Ok(format!("SSH check successful: {}:{}", hostname, spec.port))
}

pub async fn perform_check(hostname: &str, check_spec: &CheckSpec) -> Result<String> {
    match check_spec {
        CheckSpec::Http(spec) => perform_http_check(hostname, spec).await,
        CheckSpec::Icmp(spec) => perform_icmp_check(hostname, spec),
        CheckSpec::Ssh(spec) => perform_ssh_check(hostname, spec),
        CheckSpec::Nix(spec) => perform_nix_check(hostname, spec),
    }
}
