<script setup lang="ts">
import { InformationCircleIcon, CodeBracketIcon } from '@heroicons/vue/24/outline'
</script>

<template>
  <div class="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
    <div class="mb-8">
      <div class="flex items-center">
        <InformationCircleIcon class="h-8 w-8 text-black mr-3" />
        <div>
          <h1 class="text-3xl font-bold text-subheading">About CARVE</h1>
          <p class="text-body mt-1">Platform information and licensing</p>
        </div>
      </div>
    </div>

    <div class="space-y-8">
      <!-- Platform Information -->
      <div class="card p-6">
        <h2 class="text-xl font-semibold text-body mb-4">Platform Overview</h2>
        <div class="prose prose-sm max-w-none text-body">
          <p>
            CARVE is a cybersecurity competition platform designed to facilitate red team vs. blue team exercises 
            and capture-the-flag (CTF) competitions. The platform provides real-time scoring, team management, 
            and comprehensive monitoring capabilities.
          </p>
          <p>
            Built for educational and training purposes, CARVE enables organizations to run cybersecurity 
            competitions that test both offensive and defensive security skills in a controlled environment.
          </p>
        </div>
      </div>

      <!-- Features -->
      <div class="card p-6">
        <h2 class="text-xl font-semibold text-body mb-4">Key Features</h2>
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div class="space-y-3">
            <div class="flex items-start">
              <div class="w-2 h-2 card rounded-full mt-2 mr-3 flex-shrink-0"></div>
              <div>
                <h3 class="font-medium text-body">Real-time Scoring</h3>
                <p class="text-sm text-body">Live updates of team scores and rankings</p>
              </div>
            </div>
            <div class="flex items-start">
              <div class="w-2 h-2 card rounded-full mt-2 mr-3 flex-shrink-0"></div>
              <div>
                <h3 class="font-medium text-body">Team Management</h3>
                <p class="text-sm text-body">Organized team structure and member tracking</p>
              </div>
            </div>
            <div class="flex items-start">
              <div class="w-2 h-2 card rounded-full mt-2 mr-3 flex-shrink-0"></div>
              <div>
                <h3 class="font-medium text-body">OIDC Authentication</h3>
                <p class="text-sm text-body">Secure single sign-on integration</p>
              </div>
            </div>
          </div>
          <div class="space-y-3">
            <div class="flex items-start">
              <div class="w-2 h-2 bg-white rounded-full mt-2 mr-3 flex-shrink-0"></div>
              <div>
                <h3 class="font-medium text-body">Event Monitoring</h3>
                <p class="text-sm text-body">Comprehensive logging and event tracking</p>
              </div>
            </div>
            <div class="flex items-start">
              <div class="w-2 h-2 bg-white rounded-full mt-2 mr-3 flex-shrink-0"></div>
              <div>
                <h3 class="font-medium text-body">Leaderboard</h3>
                <p class="text-sm text-body">Dynamic rankings and competition standings</p>
              </div>
            </div>
            <div class="flex items-start">
              <div class="w-2 h-2 bg-white rounded-full mt-2 mr-3 flex-shrink-0"></div>
              <div>
                <h3 class="font-medium text-body">Modern Web UI</h3>
                <p class="text-sm text-body">Responsive design built with Vue.js</p>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Technology Stack -->
      <div class="card p-6">
        <h2 class="text-xl font-semibold text-body mb-4">Technology Stack</h2>
        <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
          <div>
            <h3 class="font-medium text-body mb-3">Frontend</h3>
            <ul class="space-y-2 text-body">
              <li>• Vue.js 3 with TypeScript</li>
              <li>• TailwindCSS for styling</li>
              <li>• Vue Router for navigation</li>
              <li>• Heroicons for icons</li>
              <li>• Vite for build tooling</li>
            </ul>
          </div>
          <div>
            <h3 class="font-medium text-body mb-3">Backend</h3>
            <ul class="space-y-2 text-body">
              <li>• Rust with Actix Web</li>
              <li>• Redis for data storage</li>
              <li>• OAuth2/OIDC authentication</li>
              <li>• Docker containerization</li>
              <li>• RESTful API design</li>
            </ul>
          </div>
        </div>
      </div>

      <!-- License and Source -->
      <div class="card p-6">
        <div class="flex items-start">
          <CodeBracketIcon class="h-6 w-6 text-heading mr-3 mt-1 flex-shrink-0" />
          <div class="flex-1">
            <h2 class="text-xl font-semibold text-heading mb-4">Open Source License</h2>
            <div class="prose prose-sm max-w-none text-body mb-4">
              <p>
                CARVE is licensed under the <strong>GNU Affero General Public License v3.0 (AGPL-3.0)</strong>.
                This ensures that the software remains free and open source, and that any modifications or 
                improvements are shared back with the community.
              </p>
            </div>
            
            <div class="sub-card rounded-lg p-4 mb-4">
              <h3 class="font-medium text-subheading mb-2">Key License Points:</h3>
              <ul class="text-body space-y-1">
                <li>• Free to use, modify, and distribute</li>
                <li>• Source code must remain available</li>
                <li>• Network use requires source disclosure</li>
                <li>• Modifications must use compatible license</li>
              </ul>
            </div>
            
            <div class="flex items-center space-x-4">
              <a 
                href="https://github.com/your-org/carve" 
                target="_blank" 
                rel="noopener noreferrer"
                class="btn-primary inline-flex items-center"
              >
                <CodeBracketIcon class="h-4 w-4 mr-2" />
                View Source Code
              </a>
              <a 
                href="https://www.gnu.org/licenses/agpl-3.0.html" 
                target="_blank" 
                rel="noopener noreferrer"
                class="btn-secondary inline-flex items-center"
              >
                Read License
              </a>
            </div>
          </div>
        </div>
      </div>

      <!-- Version Information -->
      <div class="card p-6">
        <h2 class="text-xl font-semibold text-gray-900 mb-4">Version Information</h2>
        <div class="grid grid-cols-1 md:grid-cols-3 gap-4 text-sm">
          <div>
            <span class="font-medium text-gray-700">Platform Version:</span>
            <div class="text-gray-600 font-mono">v1.0.0</div>
          </div>
          <div>
            <span class="font-medium text-gray-700">Build Date:</span>
            <div class="text-gray-600">{{ new Date().toLocaleDateString() }}</div>
          </div>
          <div>
            <span class="font-medium text-gray-700">Environment:</span>
            <div class="text-gray-600">Production</div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
