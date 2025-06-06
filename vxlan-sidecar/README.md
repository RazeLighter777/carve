# vxlan-sidecar
# CARVE VXLAN sidecar, used to connect CARVE to the VTEP.

Uses the vtep endpoint specified in the CARVE configuration competition.yaml.

Creates a VXLAN interface with the VXLAN ID specified by the environment variable VXLAN_ID and the CID specified by the environment variable CIDR

The VXLAN interface is created with the name vxlan0.

Additionally, the sidecar will create a bridge interface named br0 and add the vxlan0 interface to it.


