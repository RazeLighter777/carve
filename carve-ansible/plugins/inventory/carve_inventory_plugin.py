from __future__ import (absolute_import, division, print_function)
__metaclass__ = type

DOCUMENTATION = r'''
    name: carve_inventory_plugin
    plugin_type: inventory
    short_description: Returns Ansible inventory competition.yaml, pulling ssh keys from Carve's redis database
    description: Returns Ansible inventory from competition.yaml
    options:
      path_to_inventory:
        description: Directory location of the inventory
        required: true
      competition_name:
        description: Name of the competition
        required: true
      ssh_proxy:
        description: SSH proxy to use for connecting to the boxes
        required: false
        default: None
      api_host:
        description: Url of the Carve API server
        required: true
      secret_key:
        description: Secret key for the Carve API server
        required: true
'''

from ansible.plugins.inventory import BaseInventoryPlugin
from ansible.errors import AnsibleError, AnsibleParserError
import yaml
import redis
import traceback


class InventoryModule(BaseInventoryPlugin):
    NAME = 'carve_inventory_plugin'

    def verify_file(self, path):
        print(f"Verifying file: {path}")
        if path.endswith(('carve_inventory.yaml', 'carve_inventory.yml')):
            return True
        return False


    def parse(self, inventory, loader, path, cache=True):
        super(InventoryModule, self).parse(inventory, loader, path)
        self._read_config_data(path)
        self.path_to_inventory = self.get_option('path_to_inventory')
        self.competition_name = self.get_option('competition_name')
        self.ssh_proxy = self.get_option('ssh_proxy')
        self.inventory.set_variable("all", "secret_key", self.get_option('secret_key'))
        self.inventory.set_variable("all", "api_host", self.get_option('api_host'))
        try:
            with open(self.path_to_inventory) as f:
                data = yaml.safe_load(f)
                # get the correct competition entry from the list
                competition = next((comp for comp in data.get('competitions', []) if comp['name'] == self.competition_name), None)
                # try the second type (helm chart)
                if not competition:
                    print("helm chart detected, using competition entry")
                    competition = data.get('competition')
                if not competition:
                    raise AnsibleParserError(f"Competition {self.competition_name} not found in inventory file {self.path_to_inventory}")
                # get the redis connection details from the competition entry
                #redis_host = competition.get('redis', {}).get('host', 'localhost')
                redis_host = "localhost"
                redis_port = competition.get('redis', {}).get('port', 6379)
                redis_db = competition.get('redis', {}).get('db', 0)
                self.redis_client = redis.StrictRedis(host=redis_host, port=redis_port, db=redis_db)
                # iterate through boxes and teams. 
                # key for the username / password can be found in the redis key {competition_name}:{team_name}:{box_name}:credentials
                # value is seperated by a colon, e.g. "user:password"
                # create group for each team and box type.
                # generated hostnames are {box_name}.{team_name}.{competition_name}.hack
                for box in competition.get('boxes', []):
                    print(f"Processing box: {box}")
                    box_name = box['name']
                    self.inventory.add_group(box_name)
                    for team in competition.get('teams', []):
                        print(f"  Processing team: {team}")
                        team_name = team['name']
                        hostname = f"{box_name}.{team_name}.{self.competition_name}.hack"
                        username = None
                        password = None
                        try:
                            credentials_key = f"{self.competition_name}:{team_name}:{box_name}:credentials"
                            credentials = self.redis_client.get(credentials_key)
                            if credentials:
                                username, password = credentials.decode().split(':')
                            else:
                                continue
                        except Exception as e:
                            continue
                        ip_address = None
                        # get the IP address using the redis key {competition_name}:{team_name}:{box_name}:ip_address
                        ip_address_key = f"{self.competition_name}:{team_name}:{box_name}:ip_address"
                        ip_address = self.redis_client.get(ip_address_key)
                        if not ip_address:
                            print(f"    No IP address found for {hostname}, skipping")
                            continue
                        ip_address = ip_address.decode()
                        print(f"    Adding host: {hostname} with user: {username}")
                        print("    team_name:", team_name)
                        print("    box_name:", box_name)
                        self.inventory.add_host(hostname, group=box_name)
                        self.inventory.set_variable(hostname, 'ansible_ssh_common_args', f'-o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -J {self.ssh_proxy}')
                        self.inventory.set_variable(hostname, 'ansible_host', ip_address)
                        self.inventory.set_variable(hostname, 'ansible_user', username)
                        self.inventory.set_variable(hostname, 'ansible_password', password)

        except Exception as e:
            traceback.print_exc()
            raise AnsibleParserError(f"Failed to parse inventory file {path}: {e}")