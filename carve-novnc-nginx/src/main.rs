use std::process::exit;

use carve::{config::AppConfig, redis_manager};

fn main() {
    let config = AppConfig::new().expect("Failed to load configuration");
    let competition = &config.competitions[0];
    let redis_manager =
        redis_manager::RedisManager::new(&competition.redis).expect("Failed to connect to Redis");
    let mut nginx_config = "# Nginx configuration for Carve competition\n\
    map $http_upgrade $connection_upgrade { \
        default upgrade; \
        '' close; \
}\n \
 \
server { \
        listen  80 default_server; \
        keepalive_timeout       70;"
        .to_string();
    // pull resolver from /etc/resolv.conf
    let mut resolver_address = String::new();
    // check if dns_upstream_service
    if let Some(dns_upstream_service) = competition.dns_upstream_service.clone() {
        resolver_address = dns_upstream_service;
    } 
    else if let Some(contents) = std::fs::read_to_string("/etc/resolv.conf").ok() {
        // Extract the first nameserver line
        for line in contents.lines() {
            if line.starts_with("nameserver") {
                // Split the line and take the second part (the IP address)
                if let Some(ip) = line.split_whitespace().nth(1) {
                    resolver_address = ip.to_string();
                    break;
                } else {
                    eprintln!("Failed to parse IP address from /etc/resolv.conf");
                    exit(1);
                }
            }
        }
    }
    //loop through each team and box
    for team in &competition.teams {
        // get the teams console password from redis
        let console_password = redis_manager
            .get_box_console_code(&competition.name, &team.name)
            .expect("Failed to get team console password");
        for b in &competition.boxes {
            nginx_config += &format!(
                "location /novnc/{}/{}-{} {{ \
                   resolver {} valid=10s; \
                   error_log /var/log/nginx/novnc.log notice; \
                   proxy_pass http://vxlan-sidecar-{}-{}:5700; \
                   rewrite_log on; \
                   rewrite ^/novnc/{}/{}-{}(/.*)?$ /$1 break; \
                   proxy_http_version 1.1; \
                   proxy_set_header Upgrade $http_upgrade; \
                   proxy_set_header Connection $connection_upgrade; \
                   proxy_set_header Host $host; \
                   proxy_set_header X-Real-IP $remote_addr; \
                   proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for; \
               }}\n",
                console_password,
                team.name,
                b.name,
                resolver_address,
                team.name,
                b.name,
                console_password,
                team.name,
                b.name
            );
        }
    }
    nginx_config += "}\n";

    // Write the configuration to a file
    std::fs::write("/etc/nginx/sites-enabled/carve", nginx_config.clone())
        .expect("Failed to write Nginx configuration file");
    // print configuration to console
    println!("Generated Nginx configuration:\n{}", nginx_config);
    //start nginx, and wait for it to finish
    let status = std::process::Command::new("nginx")
        .status()
        .expect("Failed to start Nginx");
    if !status.success() {
        eprintln!("Failed to start Nginx: {}", status);
    } else {
        println!("Nginx started successfully");
    }
    // Print success message
    println!("Nginx configuration generated and server started successfully.");
    // Keep the main thread alive to prevent the program from exiting
    loop {
        std::thread::park();
    }
}
