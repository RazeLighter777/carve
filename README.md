# CARVE (Cyberspace Assessment Range for Virtual Environments)

## Containers
### canary : Conduct scoring checks for competitions. 
### carve : CARVE Library, handles configuration.
### vtep : CARVE VTEP, creates VXLAN tunnels for CARVE and routes traffic.
### vxlan-sidecar : CARVE VXLAN sidecar, used to connect CARVE to the VTEP.
### redis : COT redis container
### carve-web : vue + react 
### qemu-box : Runs the actual VM. Requires passthrough of KVM and tun devices. Exposes VM on vnc.