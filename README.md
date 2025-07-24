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
- **Automatic Infrastructure Management**: Let CARVE handle the tedious work of making networks and virtual machines. Unlike many CTFs, carve gives players VMs (not containers) expanding the range of possibilities. And the network managed network is isolated, secure, and created automatically with the magic of VXLANs. 
- **Performance** CARVE is built to maximize the capabilities of the underlying hardware. It scales horizontally, and uses QEMU backing images and snapshots to minimize disk space requirements. It feels snappy. 
- **Virtual Networking:** Uses VXLANs and dnsmasq to create isolated, team-specific networks. All the networking is handled by CARVE, with no need for manual setup.
- **Cloud-Init & Ansible:** VMs are provisioned with cloud-init for basic setup, then made vulnerable for the game with Ansible playbooks.
- **Modern Frontend:** Built with TypeScript, Vue, and Tailwind CSS.
- **Rust Backend:** All backend services are written in Rust. Why? It's the language I know the best.
- **Redis Persistence:** CARVE uses redis as the sole database / backend communication tool. Its easy to deploy and makes everything snappy. (and contrary to popular belief won't lose all data if the RAM gives out)
- **OIDC Authentication:** Currently supports OIDC and local user/password authentication.
- **Docker Compose Deployment:** Easy to run locally for testing.
- **Helm chart support:** Helm chart makes deploying in production easy. 

## 🧱 Architecture

- **canary:** Conducts scoring checks for competitions.
- **carve:** Core library for configuration and shared logic.
- **vtep:** Creates VXLAN tunnels and routes traffic.
- **vxlan-sidecar:** Connects CARVE to the VTEP.
- **redis:** In-memory database for persistence.
- **carve-web:** Vue frontend for user interaction.
- **qemu-box:** Runs VMs, exposes them via VNC and a raw serial interface with websockets. 
- **carve-dnsmasq:** DNS and DHCP server for VM boxes.
- **carve-api:** API gateway for CARVE services.
- **qemu-ndb:** Serves qemu disks over the network to maximize storage efficiency. 
## ⚠️ Requirements
Works only on linux and WSL. You need docker or k8s. Cilium in native-routing mode is the only tested CNI. 

Cilium tunnel mode does NOT work for multi node setups due to a bug in how they handle nested VXLANS. 

**The K8S Helm chart is highly recommended over the docker compose. The helm chart makes configuration much less verbose because it parses the competitions.yaml (under the competitions yaml key) and makes the containers automatically with the right environment variables. The docker compose is for testing/evaluation purposes only**
### For testing / non-production:
- 16gb RAM (so you can run a couple VMs without them running out of RAM)
- 4 cores (more the better rust takes really long to compile)
### For running an actual game (if you don't want your players to be frustrated at your slow, laggy CTF):
At least a three node K8S setup EACH with:
- RAM = 1GB * teams * players per team
-  .5 cores * teams * players per team. 

## 🟢 Getting Started

1. **Clone the repository:**
   ```bash
   git clone https://github.com/yourusername/carve.git
   cd carve
   ```

2. **Configure the platform:**
- K8S : edit your values.yaml file in charts/carve. competition.yaml is mapped to the competition section.
- docker (only for testing, not recommended): edit docker-compose.yaml and competition.yaml files. Make sure they match. 
3. **Run:**
  K8S:
   ```
   cd charts/carve
   helm install carve -f values.yaml .
   ```
   or docker compose : 
   ```bash
   docker compose build
   docker compose up
   ```
   You can read the default admin and password (set to generate and print on the first run, if configured) using
   ```bash
   kubectl get pods -n <namespace you installed the chart to, probably "default">
   kubectl logs -n <namespace> <carve api pod>
   ```
   or docker compose:
  ```bash
  docker compose logs carve-api
  ```
4. **Run Ansible Playbooks:**
   - Install the carve ansible plugin https://galaxy.ansible.com/ui/repo/published/razelighter777/carve_ansible/
   - Configure the carve_inventory.yaml file (see carve-ansible/carve_inventory.yaml for example)
   - Forward the redis port from the redis container / pod (k8s) and the ssh port from the openssh container / container in the network pod (k8s) to localhost if running in k8s.
   - Install a public ssh key on the ssh instance.
   - Run your playbooks. 
5. **Access the frontend:**
   - K8S: Make sure you have an ingress controller installed and it will configure it automatically at the hostname inside the values.yaml at the ingress.host field. Then just go to the site.
   - Docker: Open a new terminal, go to the `carve-web` directory, and run:
     ```bash
     npm install
     npm run dev
     ```
   - The frontend will be available at the address shown in the terminal output (typically http://localhost:5173).

7. **Start the Competition:**
   - Log in as an admin user (your OIDC account must be in the admin group if you are using OIDC, otherwise you can use the other default admin user `admin`)
   - In the web UI, click "Start Competition" to begin.


## Notes

- **Persistence:** Redis is used for all data storage.
- **Authentication:** OIDC and local user/password supported. OIDC is only tested with authentik right now, may or may not work with other applications.
- **Challenges:** Only a few dummy challenges are included. Add more by creating Ansible playbooks.
- **Competition Automation:** Future updates will improve Ansible integration for automatic and modular playbook creation.
- **Scalability:** Scales horizontally with K8S. 
- **Documentation:** More detailed docs for configuration and usage are coming soon.

## Contributing

The codebase is in an early, functional but "scrappy" state. Major cleanup and refactoring are needed. Contributions are welcome!

## License

This project is licensed under the AGPLv3 License.
