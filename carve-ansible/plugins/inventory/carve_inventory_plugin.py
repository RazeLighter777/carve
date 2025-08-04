from __future__ import (absolute_import, division, print_function)
__metaclass__ = type

DOCUMENTATION = r'''
    name: carve_inventory_plugin
    plugin_type: inventory
    short_description: Returns Ansible inventory from competition.yaml, pulling box information from Carve's API
    description: Returns Ansible inventory from competition.yaml using Carve API calls
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
import requests
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
        self.api_host = self.get_option('api_host')
        self.secret_key = self.get_option('secret_key')
        self.inventory.set_variable("all", "secret_key", self.secret_key)
        self.inventory.set_variable("all", "api_host", self.api_host)
        
        # Setup API headers
        self.headers = {
            'Authorization': f'Bearer {self.secret_key}',
            'Content-Type': 'application/json'
        }
        
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
                
                # iterate through boxes and teams
                # generated hostnames are {box_name}.{team_name}.{competition_name}.hack
                for box in competition.get('boxes', []):
                    print(f"Processing box: {box}")
                    box_name = box['name']
                    self.inventory.add_group(box_name)
                    for team in competition.get('teams', []):
                        print(f"  Processing team: {team}")
                        team_name = team['name']
                        hostname = f"{box_name}.{team_name}.{self.competition_name}.hack"
                        
                        # Get box details (including IP) from API
                        box_details = self._get_box_details(hostname)
                        if not box_details:
                            print(f"    No box details found for {hostname}, skipping")
                            continue
                        
                        ip_address = box_details.get('ipAddress')
                        if not ip_address or ip_address in ['Unset', 'DNS Misconfiguration']:
                            print(f"    No IP address found for {hostname}, skipping")
                            continue
                        
                        # Get box credentials from API
                        credentials = self._get_box_credentials(hostname, team_name)
                        if not credentials:
                            print(f"    No credentials found for {hostname}, skipping")
                            continue
                        
                        username = credentials.get('username')
                        password = credentials.get('password')
                        
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
    
    def _get_box_details(self, box_name):
        """Get box details including IP address from the API"""
        try:
            url = f"{self.api_host}/api/v1/internal/box"
            params = {'name': box_name}
            response = requests.get(url, headers=self.headers, params=params, timeout=30)
            print(f"response data: {response.text}")
            if response.status_code == 200:
                return response.json()
            else:
                print(f"    Failed to get box details for {box_name}: HTTP {response.status_code}")
                return None
        except requests.RequestException as e:
            print(f"    Error getting box details for {box_name}: {e}")
            return None
    
    def _get_box_credentials(self, box_name, team_name):
        """Get box credentials from the API"""
        try:
            url = f"{self.api_host}/api/v1/internal/box/creds_for"
            params = {
                'name': box_name,
                'team': team_name
            }
            response = requests.get(url, headers=self.headers, params=params, timeout=30)
            print(f"cred response data: {response.text}")
            if response.status_code == 200:
                return response.json()
            else:
                print(f"    Failed to get credentials for {box_name}: HTTP {response.status_code}")
                return None
        except requests.RequestException as e:
            print(f"    Error getting credentials for {box_name}: {e}")
            return None