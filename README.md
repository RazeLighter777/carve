# 🎃 CARVE (Cyberspace Assessment Range for Virtual Environments)
![image](https://github.com/user-attachments/assets/12919fa8-9670-470e-a940-66ec9aa0d0fd)

[Click here if you wanna see some more pretty screenshots](https://github.com/RazeLighter777/carve/wiki)

CARVE is an open source attack-defense CTF (Capture The Flag) platform. Players compete in small teams and show off their l33t skills by hacking into other teams networks to score points. At the same time, teams have to play defense to prevent their networks from being hacked.

Each team recieves access to an identical set of virtual computers (VMs) through a simple easy to use website. The VMs are literred with security flaws and holes, and unfortunately, wide open to attacks from other teams.😈 

Backend VM management, networking, scoring, authentication, administration tools, and a sleek looking website are all included!!!! And it's mostly configured through a single file `competition.yaml`.

Unlike most open-source CTF platforms out there, CARVE is designed to be easy to use and deployable with a few simple commands.

CARVE **DOES NOT** include the actual challenges and setup scripts to make a real competition. You'll need to write them yourself and keep them private until the competition is over, so you don't spoil your game.
## ✨ Features

- **Microservice Architecture:** Modular services for scoring, networking, orchestration, everything you need to run a CTF at scale.
- **Attack-Defense CTF:** Teams receive VMs to defend and attack others, earning points for solving challenges and keeping services alive.
- **Virtual Networking:** Uses VXLANs and dnsmasq to create isolated, team-specific networks. All the networking is handled by CARVE, with no need for manual setup.
- **Cloud-Init & Ansible:** VMs are provisioned with cloud-init for basic setup, then made vulnerable for the game with Ansible playbooks.
- **Modern Frontend:** Built with TypeScript, Vue, and Tailwind CSS.
- **Rust Backend:** All backend services are written in Rust. Why? It's the language I know the best.
- **Redis Persistence:** CARVE uses redis as the sole database / backend communication tool. Its easy to deploy and makes everything snappy. (and contrary to popular belief won't lose all data if the RAM gives out)
- **OIDC Authentication:** Currently supports OIDC and local user/password authentication.
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
## ⚠️ Requirements
Works only on linux and WSL. You need docker or any OCI container platform. 

### For testing / non-production:
- 16gb RAM (so you can run a couple VMs without them running out of RAM)
- 4 cores (more the better rust takes really long to compile)
### For running an actual game (if you don't want your players to be frustrated at your slow, laggy CTF):
At least a three node (docker, or kubernetes) setup EACH with:
- RAM = 1GB * teams * players per team
-  .5 cores * teams * players per team. 

## 🟢 Getting Started

1. **Clone the repository:**
   ```bash
   git clone https://github.com/yourusername/carve.git
   cd carve
   ```

2. **Configure the platform:**
   - Edit `docker-compose.yaml` to set your OIDC credentials (optional) and `SECRET_KEY` 
   - Edit `competition.yaml` to define your competition (docs coming soon).
   - Place your VM disk images in `disks/<subdirectory>/`.
   - Add your Ansible playbooks in the `carve-ansible` directory.

3. **Build and run with Docker Compose:**
   ```bash
   docker compose build
   docker compose up
   ```
   You can read the default admin and password (set to generate and print on the first run, if configured) using
   ```bash
   docker compose logs carve-api
   ```

5. **Run Ansible Playbooks:**
   - In a new terminal, exec into the carve-ansible container:
     ```bash
     docker compose exec carve-ansible bash
     ```
   - Run your playbook (replace with your playbook path):
     ```bash
     uv run ansible-playbook playbook.yaml -i carve_inventory.yaml
     ```

6. **Access the frontend:**
   - Open a new terminal, go to the `carve-web` directory, and run:
     ```bash
     npm install
     npm run dev
     ```
   - The frontend will be available at the address shown in the terminal output (typically http://localhost:5173).

7. **Start the Competition:**
   - Log in as an admin user (your OIDC account must be in the admin group).
   - In the web UI, click "Start Competition" to begin.

## Notes

- **Persistence:** Redis is used for all data storage. By default, persistence is disabled for testing. AOF and backup configuration will be added soon.
- **Authentication:** OIDC and local user/password supported. OIDC is only tested with authentik right now, may or may not work with other applications.
- **Challenges:** Only a few dummy challenges are included. Add more by creating Ansible playbooks.
- **Competition Automation:** Future updates will improve Ansible integration for automatic and modular playbook creation.
- **Scalability:** Currently runs with Docker Compose (single host). Helm charts for Kubernetes are planned for horizontal scaling.
- **Documentation:** More detailed docs for configuration and usage are coming soon.

## Contributing

The codebase is in an early, functional but "scrappy" state. Major cleanup and refactoring are needed. Contributions are welcome!

## License

This project is licensed under the AGPLv3 License.
