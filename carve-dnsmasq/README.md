# carve-dnsmasq

carve-dnsmasq is a DNS and DHCP server for CARVE boxes.

It runs in the same pod as the VTEP and provides DNS and DHCP services for the CARVE vxlan network. It uses dnsmasq to provide these services.

It takes an environment variable COMPETITION_NAME to determine the competition name and uses it to find the correct entry for the competition in the configuration file.

It finds the specific competition entry in the "competitions" array with COMPETITION_NAME from the configuration file (/config/competition.yaml) and uses it to generate the DNS and DHCP configuration with the entry.sh script.

To generate this configuration, it uses yq to parse the configuration file and generate the dnsmasq configuration file.

Steps to make the dnsmasq configuration:

1. Parse the configuration file to find the competition entry.
2. Add the global dnsmasq configuration to the dnsmasq configuration file. This includes:
   - Setting the domain to the competition name.
   - configuring listening interfaces.
    ```
    except-interface=lo # Exclude the loopback interface
    except-interface=eth0 # Exclude the eth0 interface so we don't hand out IPs to the container network
    domain=<competition_name>.local # Set the domain to the competition name
    no-resolv # Don't read /etc/resolv.conf
    ```
3. For every team name:
    - Compute the CIDR (Classless Inter-Domain Routing) for the team's network. Remember that the VTEP uses a /24 network for each team, subnetted from the /16 network in the compeititions cidr yaml field. The first /24 is reserved for management, and the rest are assigned to teams in order of their index in the configuration file.
    - Generate the dnsmasq configuration for the team's network, appending it to the dnsmasq configuration file.
      Here is an example of the dnsmasq configuration for a team:
      ```
      dhcp-range=10.13.1.16,10.13.1.253,255.255.255.0,12h # DHCP range for the team. starts at .16 and ends at .253 to avoid conflicts with the VTEP
      dhcp-option=option:router,10.13.1.254 # VTEP IP always .254
      dhcp-option=option:dns-server,10.13.1.254 # carve-dnsmasq is listening on the VTEP IP
      dhcp-authoritative # Make dnsmasq authoritative for this network
      ```
4. Write the dnsmasq configuration file to /etc/dnsmasq.conf.
