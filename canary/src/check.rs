use anyhow::{Context, Result};
use log::{debug, error, info};
use regex::Regex;
use ssh2::Session;
use std::net::TcpStream;
use std::time::Duration;

use carve::config::{CheckSpec, HttpCheckSpec, HttpMethods, IcmpCheckSpec, SshCheckSpec};

pub async fn perform_http_check(hostname: &str, spec: &HttpCheckSpec) -> Result<String> {
    debug!(
        "Starting HTTP check for host: {} with spec: {:?}",
        hostname, spec
    );
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
        .header(
            "Content-Type",
            if spec.method == HttpMethods::Post {
                "application/x-www-form-urlencoded"
            } else {
                "application/json"
            },
        )
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
    debug!("HTTP response status: {}, body: {}", status, body);

    if status.as_u16() != spec.code {
        error!(
            "HTTP status code mismatch: expected {}, got {}",
            spec.code,
            status.as_u16()
        );
        return Err(anyhow::anyhow!(
            "HTTP status code mismatch: expected {}, got {}",
            spec.code,
            status.as_u16()
        ));
    }

    let re = Regex::new(&spec.regex).context("Invalid regex pattern")?;
    if !re.is_match(&body) {
        error!("Response body does not match regex: {}", spec.regex);
        return Err(anyhow::anyhow!(
            "Response body does not match regex: {}",
            spec.regex
        ));
    }

    info!("HTTP check successful for {}", url);
    Ok(format!("HTTP check successful: {}", url))
}

pub async fn perform_nix_check(
    hostname: &str,
    spec: &carve::config::NixCheckSpec,
) -> Result<String> {
    debug!(
        "Starting Nix check for host: {} with spec: {:?}",
        hostname, spec
    );

    // Execute the script directly with bash -c
    let script_with_hostname = format!("{} {}", spec.script, hostname);
    let p = tokio::process::Command::new("nix-shell")
        .arg("-p")
        .args(
            spec.packages
                .as_ref()
                .map(|pkgs| pkgs.iter().map(String::as_str).collect::<Vec<_>>())
                .unwrap_or_default(),
        )
        .kill_on_drop(true)
        .arg("--run")
        .arg(format!("bash -c '{}'", script_with_hostname))
        .output();
    let output = tokio::time::timeout(Duration::from_secs(spec.timeout), p);
    let output = match output.await {
        Ok(res) => {
            debug!("Nix check output: {:?}", res);
            res.context("Failed to execute Nix check command")
                .map_err(|e| anyhow::anyhow!("Nix check command failed: {}", e))?
        }
        Err(e) => {
            error!("Nix check command timed out after {}", e);

            return Err(anyhow::anyhow!("Nix check command timed out"));
        }
    };

    if output.status.success() {
        info!("Nix check successful for host: {}", hostname);
        Ok(format!(
            "Nix check successful: {}",
            String::from_utf8_lossy(&output.stdout)
        ))
    } else {
        error!(
            "Nix check failed for host: {}: {}",
            hostname,
            String::from_utf8_lossy(&output.stderr)
        );
        Err(anyhow::anyhow!(
            "Nix check failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

pub fn perform_icmp_check(hostname: &str, spec: &IcmpCheckSpec) -> Result<String> {
    debug!(
        "Starting ICMP check for host: {} with spec: {:?}",
        hostname, spec
    );
    // Simplify the ICMP check for now
    // In a real implementation, we would use proper DNS resolution and error handling
    // This is a simplified version that just checks if the host responds to ping

    // Use hostname directly (the ping library will handle resolution)
    let result = std::process::Command::new("ping")
        .args(["-c", "1", "-W", "5", hostname])
        .output()
        .context("Failed to execute ping command")?;
    debug!("Ping command executed for host: {}", hostname);

    let success = result.status.success();

    // Check if the result matches our expectation
    if (success && spec.code == 0) || (!success && spec.code != 0) {
        info!("ICMP check successful for host: {}", hostname);
        Ok(format!("ICMP check successful: {}", hostname))
    } else {
        error!(
            "ICMP check failed for host: {}: expected code {}, got {}",
            hostname,
            spec.code,
            if success { 0 } else { 1 }
        );
        Err(anyhow::anyhow!(
            "ICMP check failed: expected code {}, got {}",
            spec.code,
            if success { 0 } else { 1 }
        ))
    }
}

pub fn perform_ssh_check(hostname: &str, spec: &SshCheckSpec) -> Result<String> {
    debug!(
        "Starting SSH check for host: {} with spec: {:?}",
        hostname, spec
    );
    let tcp = TcpStream::connect(format!("{}:{}", hostname, spec.port))
        .context("Failed to connect to SSH server")?;
    debug!("TCP connection established for SSH check");

    let mut session = Session::new().context("Failed to create SSH session")?;
    session.set_tcp_stream(tcp);
    session.handshake().context("Failed SSH handshake")?;
    debug!("SSH handshake completed");

    if let Some(password) = &spec.password {
        session
            .userauth_password(&spec.username, password)
            .context("SSH authentication failed")?;
        debug!("SSH password authentication attempted");
    } else if let Some(key_path) = &spec.key_path {
        session
            .userauth_pubkey_file(&spec.username, None, std::path::Path::new(key_path), None)
            .context("SSH key authentication failed")?;
        debug!("SSH key authentication attempted");
    } else {
        error!(
            "No SSH authentication method provided for host: {}",
            hostname
        );
        return Err(anyhow::anyhow!("No SSH authentication method provided"));
    }

    if !session.authenticated() {
        error!("SSH authentication failed for host: {}", hostname);
        return Err(anyhow::anyhow!("SSH authentication failed"));
    }

    info!("SSH check successful for host: {}:{}", hostname, spec.port);
    Ok(format!("SSH check successful: {}:{}", hostname, spec.port))
}

pub async fn perform_check(hostname: &str, check_spec: &CheckSpec) -> Result<String> {
    debug!(
        "Dispatching check for host: {} with spec: {:?}",
        hostname, check_spec
    );
    match check_spec {
        CheckSpec::Http(spec) => perform_http_check(hostname, spec).await,
        CheckSpec::Icmp(spec) => perform_icmp_check(hostname, spec),
        CheckSpec::Ssh(spec) => perform_ssh_check(hostname, spec),
        CheckSpec::Nix(spec) => perform_nix_check(hostname, spec).await,
    }
}
