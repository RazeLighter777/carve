# VTEP: VXLAN Tunnel Endpoint for CARVE

VTEP (Virtual Tunnel Endpoint) is a component of the CARVE (Cyberspace Assessment Range for Virtual Environments) project that creates VXLAN tunnels for CARVE and routes traffic.

It reads the configuration from competition.yaml and allocated each team a /24 from the /16 subnet defined in the configuration. It first allocates a /24 for management services (like the scoring checks). It then creates a VXLAN tunnel for each team and routes traffic to the appropriate team based on the VXLAN ID.

All traffic is SNATed so that it appears to come from the VTEP MGMT address /24 (using SNAT ranges). This allows teams to communicate with each other and with the CARVE infrastructure without exposing their internal IP addresses, denying them the ability to block traffic based on IP addresses.

The subnets are stored in a redis hash with the format `<competition name>:subnets`, where each key is a team name and the values are the allocated subnet in CIDR notation, the team name (or MGMT) and the VXLAN ID. This is cleaned when vtep starts to ensure no stale data is present.

The container required NET_ADMIN capabilities to create VXLAN interfaces and manage routing.