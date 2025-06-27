# 🎃 CARVE (Cyberspace Assessment Range for Virtual Environments)
![image](https://github.com/user-attachments/assets/12919fa8-9670-470e-a940-66ec9aa0d0fd)

[Click here if you wanna see some more pretty screenshots](https://github.com/RazeLighter777/carve/wiki)

CARVE is an open source, microservice-based attack-defense CTF (Capture The Flag) platform designed for cybersecurity competitions. It enables teams to compete by attacking and defending virtual machines (VMs) in a realistic, isolated network environment.

## ✨ Features

- **Microservice Architecture:** Modular services for scoring, networking, orchestration, and more.
- **Attack-Defense CTF:** Teams receive VMs to defend and attack others, earning points for solving challenges and keeping services alive.
- **Virtual Networking:** Uses VXLANs and dnsmasq to create isolated, team-specific networks.
- **Cloud-Init & Ansible:** VMs are provisioned with cloud-init for basic setup, then made vulnerable with Ansible playbooks.
- **Modern Frontend:** Built with TypeScript, Vue, and Tailwind CSS.
- **Rust Backend:** All backend services are written in Rust for performance and safety.
- **Redis Persistence:** Uses Redis as the sole database (AOF and backup configuration coming soon).
- **OIDC Authentication:** Currently supports OIDC only; more auth methods planned.
- **Docker Compose Deployment:** Easy to run locally; Kubernetes (Helm) support coming soon.

## 🧱 Architecture

- **canary:** Conducts scoring checks for competitions.
- **carve:** Core library for configuration and shared logic.
- **vtep:** Creates VXLAN tunnels and routes traffic.
- **vxlan-sidecar:** Connects CARVE to the VTEP.
- **redis:** In-memory database for persistence.
- **carve-web:** Vue frontend for user interaction.
- **qemu-box:** Runs VMs, exposes them via VNC.
- **carve-dnsmasq:** DNS and DHCP server for VM boxes.
- **carve-api:** API gateway for CARVE services.
- **carve-ansible:** Integrates Ansible for VM provisioning and challenge deployment.

## 🟢 Getting Started (good luck btw everything is very hacky rn)

1. **Clone the repository:**
   ```bash
   git clone https://github.com/yourusername/carve.git
   cd carve
   ```

2. **Configure the platform:**
   - Edit `docker-compose.yaml` to set your OIDC credentials and `SECRET_KEY`.
   - Edit `competition.yaml` to define your competition (docs coming soon).
   - Place your VM disk images in `disks/<subdirectory>/`.
   - Add your Ansible playbooks in the `carve-ansible` directory.

3. **Build and run with Docker Compose:**
   ```bash
   docker compose build
   docker compose up
   ```

4. **Run Ansible Playbooks:**
   - In a new terminal, exec into the carve-ansible container:
     ```bash
     docker compose exec carve-ansible bash
     ```
   - Run your playbook (replace with your playbook path):
     ```bash
     uv run ansible-playbook playbook.yaml -i carve_inventory.yaml
     ```

5. **Access the frontend:**
   - Open a new terminal, go to the `carve-web` directory, and run:
     ```bash
     npm install
     npm run dev
     ```
   - The frontend will be available at the address shown in the terminal output (typically http://localhost:5173).

6. **Start the Competition:**
   - Log in as an admin user (your OIDC account must be in the admin group).
   - In the web UI, click "Start Competition" to begin.

## Notes

- **Persistence:** Redis is used for all data storage. By default, persistence is disabled for testing. AOF and backup configuration will be added soon.
- **Authentication:** Only OIDC is supported right now. More authentication methods are planned.
- **Challenges:** Only a few dummy challenges are included. Add more by creating Ansible playbooks.
- **Competition Automation:** Future updates will improve Ansible integration for automatic and modular playbook execution.
- **Scalability:** Currently runs with Docker Compose (single host). Helm charts for Kubernetes are planned for horizontal scaling.
- **Documentation:** More detailed docs for configuration and usage are coming soon.

## Contributing

The codebase is in an early, functional but "scrappy" state. Major cleanup and refactoring are needed. Contributions are welcome!

## License

This project is licensed under the AGPLv3 License.
