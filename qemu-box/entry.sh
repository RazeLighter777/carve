#!/bin/bash
set -e

# Parse environment variables
if [ -z "$COMPETITION_NAME" ] || [ -z "$BOX_NAME" ] || [ -z "$TEAM_NAME" ]; then
    echo "Error: Required environment variables not set"
    echo "Please set: COMPETITION_NAME, BOX_NAME, TEAM_NAME"
    exit 1
fi

echo "Starting qemu-box for competition: $COMPETITION_NAME, box: $BOX_NAME, team: $TEAM_NAME"

# Configuration file path
CONFIG_FILE="/config/competition.yaml"

if [ ! -f "$CONFIG_FILE" ]; then
    echo "Error: Configuration file not found at $CONFIG_FILE"
    exit 1
fi

# Find the first qcow2 image in /disk directory
DISK_IMAGE=$(find /disk -name "*.qcow2" | head -n 1)

if [ -z "$DISK_IMAGE" ]; then
    echo "Error: No .qcow2 disk image found in /disk directory"
    exit 1
fi

echo "Using disk image: $DISK_IMAGE"

# Generate cloud-init user-data
CLOUD_INIT_DIR="/cloud-init"
USER_DATA_FILE="$CLOUD_INIT_DIR/user-data"

# Create cloud-init user-data
cat > "$USER_DATA_FILE" << EOF
#cloud-config
network:
  version: 1
  config:
  - type: physical
    name: eth0
    subnets:
      - type: dhcp
fqdn: ${BOX_NAME}.${TEAM_NAME}.${COMPETITION_NAME}.local
hostname: ${BOX_NAME}
prefer_fqdn_over_hostname: true
create_hostname_file: true

# Basic system configuration
users:
  - name: ubuntu
    sudo: ALL=(ALL) NOPASSWD:ALL
    shell: /bin/bash
    lock_passwd: false
    passwd:\$1\$AcnoflHT\$YW4fwKceg2eca5B7Rs0.4/

package_update: true
package_upgrade: true

runcmd:
  - systemctl enable ssh
  - systemctl start ssh
EOF

# Create meta-data (minimal)
cat > "$CLOUD_INIT_DIR/meta-data" << EOF
instance-id: ${BOX_NAME}-${TEAM_NAME}
local-hostname: ${BOX_NAME}
EOF

echo "Generated cloud-init configuration"

# Start HTTP server for cloud-init in background
echo "Starting HTTP server for cloud-init on port 8000"
cd "$CLOUD_INIT_DIR"
python3 -m http.server 8001 &
HTTP_SERVER_PID=$!

# Function to cleanup on exit
cleanup() {
    echo "Cleaning up..."
    if [ ! -z "$HTTP_SERVER_PID" ]; then
        kill $HTTP_SERVER_PID 2>/dev/null || true
    fi
    if [ ! -z "$QEMU_PID" ]; then
        kill $QEMU_PID 2>/dev/null || true
    fi
}

trap cleanup EXIT INT TERM

# Wait a moment for HTTP server to start
sleep 2

# Get the container's IP for cloud-init URL
CONTAINER_IP=$(hostname -I | awk '{print $1}')
echo "Container IP: $CONTAINER_IP"

# Generate /etc/qemu/bridge.conf for QEMU networking
BRIDGE_CONF="/etc/qemu/bridge.conf"
if [ ! -f "$BRIDGE_CONF" ]; then
    echo "Creating QEMU bridge configuration at $BRIDGE_CONF"
    mkdir -p /etc/qemu
    echo "allow br0" > "$BRIDGE_CONF"
fi
echo "Using QEMU bridge configuration at $BRIDGE_CONF"

# generate MAC address for the VM
MAC_ADDRESS=$(printf '52:54:00:%02x:%02x:%02x' $((RANDOM % 256)) $((RANDOM % 256)) $((RANDOM % 256)))

# configure iptables to allow traffic from the bridge
echo "Configuring iptables to allow traffic from the bridge..."
iptables -A FORWARD -i br0 -m physdev --physdev-is-bridged -j ACCEPT
# Start QEMU VM
echo "Starting QEMU VM..."
qemu-system-x86_64 \
    -enable-kvm \
    -m 1024 \
    -cpu host \
    -smp 2 \
    -drive file="$DISK_IMAGE",format=qcow2 \
    -net nic,model=virtio,macaddr=$MAC_ADDRESS \
    -net bridge,br=br0 \
    -smbios type=1,serial=ds='nocloud;\'http://$CONTAINER_IP:8001/\' \
    -display vnc=0.0.0.0:0 \
    -daemonize \
    -pidfile /tmp/qemu.pid

# Get QEMU PID
QEMU_PID=$(cat /tmp/qemu.pid)
echo "QEMU started with PID: $QEMU_PID"

# Monitor QEMU process
while kill -0 $QEMU_PID 2>/dev/null; do
    sleep 5
done

echo "QEMU process has terminated"