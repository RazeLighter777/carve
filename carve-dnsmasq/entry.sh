#!/bin/bash
set -e




# Check for COMPETITION_NAME
if [ -z "$COMPETITION_NAME" ]; then
  echo "COMPETITION_NAME environment variable not set!"
  exit 1
fi

CONFIG_FILE="/config/competition.yaml"
DNSMASQ_CONF="/etc/dnsmasq.conf"

# Check if configuration file exists
if [ ! -f "$CONFIG_FILE" ]; then
  echo "Configuration file not found at $CONFIG_FILE"
  exit 1
fi

# Get the competition CIDR
competition_cidr=$(yq ".competitions[] | select(.name == \"$COMPETITION_NAME\") | .cidr" $CONFIG_FILE)
competition_domain="$COMPETITION_NAME.local"

# Write global dnsmasq config
cat > "$DNSMASQ_CONF" <<EOF
except-interface=lo
except-interface=eth0
domain=$competition_domain
dhcp-fqdn
no-resolv
EOF

# Get teams
team_count=$(yq ".competitions[] | select(.name == \"$COMPETITION_NAME\") | .teams | length" $CONFIG_FILE)

# Calculate base network (e.g., 10.13.0.0)
base_net=$(echo $competition_cidr | cut -d'/' -f1 | tr -d \")
IFS='.' read -r o1 o2 o3 o4 <<< "$base_net"

# For each team, assign a /24 subnet (skip the first /24 for management)
for ((i=0;i<$team_count;i++)); do
  team_name=$(yq ".competitions[] | select(.name == \"$COMPETITION_NAME\").teams[$i].name" "$CONFIG_FILE" | tr -d \")
  subnet_index=$((i+1)) # skip .0 for management
  team_net="$o1.$o2.$subnet_index.0"
  dhcp_start="$o1.$o2.$subnet_index.16"
  dhcp_end="$o1.$o2.$subnet_index.253"
  router_ip="$o1.$o2.$subnet_index.254"

  cat >> "$DNSMASQ_CONF" <<TEAMCONF
  # Team: $team_name
  # Subnet: $team_net/24
  dhcp-range=set:net$i,$dhcp_start,$dhcp_end,12h
  domain=$team_name.$COMPETITION_NAME.local,$dhcp_start,$dhcp_end,12h
  dhcp-option=tag:net$i,option:router,$router_ip
  dhcp-option=tag:net$i,option:dns-server,$router_ip
  dhcp-authoritative
TEAMCONF

done

echo "dnsmasq configuration generated at $DNSMASQ_CONF"

cat $DNSMASQ_CONF


# Start dnsmasq in the foreground
dnsmasq --no-daemon --conf-file=$DNSMASQ_CONF