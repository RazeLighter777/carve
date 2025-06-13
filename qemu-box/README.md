# qemu-box

This box executes, and manages, a QEMU virtual machine. It uses shell scripts to start and stop the VM, and to manage the QEMU process.

It is required to be used with vxlan-sidecar, and uses its br0 interface to connect to the competion network.

It additionally exposes a VNC port to connect to the VM. It expects a qemu image (extension .qcow2) to be provided in the /disk directory. It will use the first image it finds in that directory.

It uses yq to parse the configuration file competition.yaml, and expects it to be in the same directory as the box.

It takes the competition name, box name, and team name as arguments, and uses them to find the configuration file (excepted in /config/configuration.yaml) and the disk image.

The box also starts a python http server to serve the cloud-init, which is generated in the entry.sh script from the configuration file.

Example cloud-init configuration:

```
#cloud-config
network:
  version: 1
  config:
  - type: physical
    name: eth0
    subnets:
      - type: dhcp
fqdn: <box_name>.<competition_name>.local
hostname: <box_name>
prefer_fqdn_over_hostname: true
create_hostname_file: true
```