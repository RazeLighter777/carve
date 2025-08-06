<script setup lang="ts">
import { ref, onMounted } from 'vue'
import apiService from '@/services/api'
import CompetitionStatus from '@/components/CompetitionStatus.vue'
import { type Team, type ApiKeysListResponse, type ToastNotification, ToastSeverity } from '@/types'

const competition = ref<any>(null)
const loading = ref(true)
const error = ref('')
const actionLoading = ref(false)
const teams = ref<Team[]>([])
const selectedTeam = ref<string>('')
const joinCode = ref<string | null>(null)
const joinCodeError = ref('')
const joinCodeLoading = ref(false)

const boxes = ref<Array<{ name: string }>>([])
const selectedBox = ref<string>('')
const boxSnapshotLoading = ref(false)
const boxSnapshotError = ref('')
const boxSnapshotSuccess = ref('')

// API Keys management
const apiKeys = ref<string[]>([])
const apiKeysLoading = ref(false)
const apiKeysError = ref('')
const creatingApiKey = ref(false)
const deletingApiKey = ref<string | null>(null)
const copiedApiKey = ref<string | null>(null)

// Toast notification publishing
const toastTitle = ref('')
const toastMessage = ref('')
const toastSeverity = ref<ToastSeverity>(ToastSeverity.Info) // Default to Info
const toastTargetType = ref<'global' | 'user' | 'team'>('global')
const toastTargetValue = ref('')
const toastPublishing = ref(false)
const toastError = ref('')
const toastSuccess = ref('')

const fetchCompetition = async () => {
  loading.value = true
  try {
    competition.value = await apiService.getCompetition()
  } catch (e) {
    error.value = 'Failed to load competition info.'
  }
  loading.value = false
}

const startCompetition = async () => {
  actionLoading.value = true
  error.value = ''
  try {
    await apiService.startCompetition()
    await fetchCompetition()
  } catch (e) {
    error.value = 'Failed to start competition.'
  }
  actionLoading.value = false
}

const endCompetition = async () => {
  actionLoading.value = true
  error.value = ''
  try {
    await apiService.endCompetition()
    await fetchCompetition()
  } catch (e) {
    error.value = 'Failed to end competition.'
  }
  actionLoading.value = false
}

const fetchTeams = async () => {
  try {
    teams.value = await apiService.getTeams()
    if (teams.value.length > 0) {
      selectedTeam.value = teams.value[0].name
    }
  } catch (e) {
    // ignore for now
  }
}

const generateJoinCode = async () => {
  joinCodeError.value = ''
  joinCode.value = null
  joinCodeLoading.value = true
  try {
    const result = await apiService.adminGenerateJoinCode({ team_name: selectedTeam.value })
    joinCode.value = result.code
  } catch (err: any) {
    joinCodeError.value = err?.response?.data?.error || 'Failed to generate join code.'
  } finally {
    joinCodeLoading.value = false
  }
}

const fetchBoxes = async () => {
  try {
    const allBoxes: Array<{ name: string }> = []
    const seen = new Set<string>()
    for (const team of teams.value) {
      const teamId = String(team.id || team.name)
      const teamBoxes = await apiService.getBoxes(teamId)
      for (const box of teamBoxes) {
        if (!seen.has(box.name)) {
          seen.add(box.name)
          allBoxes.push(box)
        }
      }
    }
    boxes.value = allBoxes
    if (boxes.value.length > 0) {
      selectedBox.value = boxes.value[0].name
    }
  } catch (e) {
    // ignore for now
  }
}

const snapshotBox = async () => {
  boxSnapshotLoading.value = true
  boxSnapshotError.value = ''
  boxSnapshotSuccess.value = ''
  try {
    await apiService.sendBoxSnapshot({ boxName: selectedBox.value })
    boxSnapshotSuccess.value = `Snapshot triggered for box: ${selectedBox.value}`
  } catch (e) {
    boxSnapshotError.value = 'Failed to snapshot box.'
  }
  boxSnapshotLoading.value = false
}

const snapshotAllBoxes = async () => {
  boxSnapshotLoading.value = true
  boxSnapshotError.value = ''
  boxSnapshotSuccess.value = ''
  try {
    for (const box of boxes.value) {
      await apiService.sendBoxSnapshot({ boxName: box.name })
    }
    boxSnapshotSuccess.value = 'Snapshots triggered for all boxes.'
  } catch (e) {
    boxSnapshotError.value = 'Failed to snapshot all boxes.'
  }
  boxSnapshotLoading.value = false
}

const fetchApiKeys = async () => {
  apiKeysLoading.value = true
  apiKeysError.value = ''
  try {
    const response = await apiService.getApiKeys()
    apiKeys.value = response.api_keys
  } catch (e) {
    apiKeysError.value = 'Failed to load API keys.'
  }
  apiKeysLoading.value = false
}

const createApiKey = async () => {
  creatingApiKey.value = true
  apiKeysError.value = ''
  try {
    await apiService.createApiKey()
    await fetchApiKeys() // Refresh the list
  } catch (e) {
    apiKeysError.value = 'Failed to create API key.'
  }
  creatingApiKey.value = false
}

const copyApiKey = async (apiKey: string) => {
  try {
    await navigator.clipboard.writeText(apiKey)
    copiedApiKey.value = apiKey
    setTimeout(() => {
      copiedApiKey.value = null
    }, 2000)
  } catch (e) {
    // Fallback for browsers without clipboard API
    const textArea = document.createElement('textarea')
    textArea.value = apiKey
    document.body.appendChild(textArea)
    textArea.focus()
    textArea.select()
    document.execCommand('copy')
    document.body.removeChild(textArea)
    copiedApiKey.value = apiKey
    setTimeout(() => {
      copiedApiKey.value = null
    }, 2000)
  }
}

const deleteApiKey = async (apiKey: string) => {
  if (!confirm(`Are you sure you want to delete this API key?\n\n${apiKey}\n\nThis action cannot be undone.`)) {
    return
  }
  
  deletingApiKey.value = apiKey
  apiKeysError.value = ''
  try {
    await apiService.deleteApiKey({ api_key: apiKey })
    await fetchApiKeys() // Refresh the list
  } catch (e) {
    apiKeysError.value = 'Failed to delete API key.'
  }
  deletingApiKey.value = null
}

const publishToast = async () => {
  toastPublishing.value = true
  toastError.value = ''
  toastSuccess.value = ''
  
  try {
    const notification: ToastNotification = {
      title: toastTitle.value,
      message: toastMessage.value,
      severity: toastSeverity.value,
      user: toastTargetType.value === 'user' ? toastTargetValue.value : undefined,
      team: toastTargetType.value === 'team' ? toastTargetValue.value : undefined
    }
    
    await apiService.publishToast(notification)
    
    // Clear form on success
    toastTitle.value = ''
    toastMessage.value = ''
    toastTargetValue.value = ''
    toastSuccess.value = 'Toast notification published successfully!'
    setTimeout(() => {
      toastSuccess.value = ''
    }, 3000)
  } catch (e) {
    toastError.value = 'Failed to publish toast notification.'
  }
  toastPublishing.value = false
}

const resetToastForm = () => {
  toastTitle.value = ''
  toastMessage.value = ''
  toastSeverity.value = ToastSeverity.Info
  toastTargetType.value = 'global'
  toastTargetValue.value = ''
  toastError.value = ''
  toastSuccess.value = ''
}

onMounted(async () => {
  await fetchCompetition()
  await fetchTeams()
  await fetchBoxes()
  await fetchApiKeys()
})
</script>

<template>
  <div class="max-w-2xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
    <h1 class="text-3xl font-bold mb-6 text-subheading">Admin Panel</h1>
    <div v-if="loading" class="text-muted">Loading...</div>
    <div v-else>
      <CompetitionStatus :competition="competition" />
      <div class="flex space-x-4 mt-6">
        <button class="btn-primary" :disabled="actionLoading" @click="startCompetition">Start Competition</button>
        <button class="btn-secondary" :disabled="actionLoading" @click="endCompetition">End Competition</button>
      </div>
      <div class="mt-8 card p-6">
        <h2 class="text-xl font-semibold mb-4">Generate Team Join Code</h2>
        <div class="flex flex-col sm:flex-row sm:items-center gap-4">
          <select v-model="selectedTeam" class="border rounded px-3 py-2">
            <option v-for="team in teams" :key="team.id" :value="team.name">{{ team.name }}</option>
          </select>
          <button class="btn-primary" :disabled="joinCodeLoading || !selectedTeam" @click="generateJoinCode">
            {{ joinCodeLoading ? 'Generating...' : 'Generate Code' }}
          </button>
        </div>
        <div v-if="joinCode" class="mt-4 text-center">
          <span class="font-mono text-lg bg-gray-100 px-3 py-1 rounded">{{ joinCode }}</span>
        </div>
        <div v-if="joinCodeError" class="text-red-600 text-center mt-2">{{ joinCodeError }}</div>
      </div>
      <!-- Box Snapshot Section -->
      <div class="mt-8 card p-6">
        <h2 class="text-xl font-semibold mb-4">Box Snapshots</h2>
        <div class="flex flex-col sm:flex-row sm:items-center gap-4">
          <select v-model="selectedBox" class="border rounded px-3 py-2">
            <option v-for="box in boxes" :key="box.name" :value="box.name">{{ box.name }}</option>
          </select>
          <button class="btn-primary" :disabled="boxSnapshotLoading || !selectedBox" @click="snapshotBox">
            {{ boxSnapshotLoading ? 'Snapshotting...' : 'Snapshot Selected Box' }}
          </button>
          <button class="btn-secondary" :disabled="boxSnapshotLoading || boxes.length === 0" @click="snapshotAllBoxes">
            {{ boxSnapshotLoading ? 'Snapshotting...' : 'Snapshot All Boxes' }}
          </button>
        </div>
        <div v-if="boxSnapshotSuccess" class="text-green-600 text-center mt-2">{{ boxSnapshotSuccess }}</div>
        <div v-if="boxSnapshotError" class="text-red-600 text-center mt-2">{{ boxSnapshotError }}</div>
      </div>
      
      <!-- Toast Notification Publishing Section -->
      <div class="mt-8 card p-6">
        <h2 class="text-xl font-semibold mb-4">Publish Toast Notification</h2>
        
        <div class="space-y-4">
          <!-- Title -->
          <div>
            <label for="toast-title" class="block text-sm font-medium text-gray-700 mb-1">Title</label>
            <input
              id="toast-title"
              v-model="toastTitle"
              type="text"
              placeholder="Notification title"
              class="w-full border border-gray-300 rounded-md px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
            />
          </div>
          
          <!-- Message -->
          <div>
            <label for="toast-message" class="block text-sm font-medium text-gray-700 mb-1">Message</label>
            <textarea
              id="toast-message"
              v-model="toastMessage"
              rows="3"
              placeholder="Notification message"
              class="w-full border border-gray-300 rounded-md px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent resize-vertical"
            ></textarea>
          </div>
          
          <!-- Severity -->
          <div>
            <label for="toast-severity" class="block text-sm font-medium text-gray-700 mb-1">Severity</label>
            <select
              id="toast-severity"
              v-model="toastSeverity"
              class="w-full border border-gray-300 rounded-md px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
            >
              <option :value="ToastSeverity.Info">Info</option>
              <option :value="ToastSeverity.Warning">Warning</option>
              <option :value="ToastSeverity.Error">Error</option>
            </select>
          </div>
          
          <!-- Target Type -->
          <div>
            <label class="block text-sm font-medium text-gray-700 mb-2">Target</label>
            <div class="space-y-2">
              <label class="flex items-center">
                <input
                  v-model="toastTargetType"
                  type="radio"
                  value="global"
                  class="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300"
                  @change="toastTargetValue = ''"
                />
                <span class="ml-2 text-sm text-gray-700">Global (all users)</span>
              </label>
              <label class="flex items-center">
                <input
                  v-model="toastTargetType"
                  type="radio"
                  value="user"
                  class="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300"
                />
                <span class="ml-2 text-sm text-gray-700">Specific user</span>
              </label>
              <label class="flex items-center">
                <input
                  v-model="toastTargetType"
                  type="radio"
                  value="team"
                  class="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300"
                />
                <span class="ml-2 text-sm text-gray-700">Specific team</span>
              </label>
            </div>
          </div>
          
          <!-- Target Value -->
          <div v-if="toastTargetType !== 'global'">
            <label :for="`toast-target-${toastTargetType}`" class="block text-sm font-medium text-gray-700 mb-1">
              {{ toastTargetType === 'user' ? 'Username' : 'Team Name' }}
            </label>
            <div v-if="toastTargetType === 'team'" class="flex gap-2">
              <select
                v-model="toastTargetValue"
                class="flex-1 border border-gray-300 rounded-md px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
              >
                <option value="">Select a team</option>
                <option v-for="team in teams" :key="team.id" :value="team.name">{{ team.name }}</option>
              </select>
            </div>
            <input
              v-else
              :id="`toast-target-${toastTargetType}`"
              v-model="toastTargetValue"
              type="text"
              :placeholder="`Enter ${toastTargetType === 'user' ? 'username' : 'team name'}`"
              class="w-full border border-gray-300 rounded-md px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
            />
          </div>
          
          <!-- Action Buttons -->
          <div class="flex gap-3">
            <button
              @click="publishToast"
              :disabled="toastPublishing || !toastTitle.trim() || !toastMessage.trim() || (toastTargetType !== 'global' && !toastTargetValue.trim())"
              class="inline-flex items-center px-4 py-2 border border-transparent text-sm font-medium rounded-md text-white bg-blue-600 hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
            >
              <svg v-if="!toastPublishing" class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 19l9 2-9-18-9 18 9-2zm0 0v-8"></path>
              </svg>
              <svg v-else class="animate-spin w-4 h-4 mr-2" fill="none" viewBox="0 0 24 24">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
              </svg>
              {{ toastPublishing ? 'Publishing...' : 'Publish Notification' }}
            </button>
            
            <button
              @click="resetToastForm"
              :disabled="toastPublishing"
              class="inline-flex items-center px-4 py-2 border border-gray-300 text-sm font-medium rounded-md text-gray-700 bg-white hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
            >
              Clear Form
            </button>
          </div>
          
          <!-- Success/Error Messages -->
          <div v-if="toastSuccess" class="text-green-600 text-sm font-medium">{{ toastSuccess }}</div>
          <div v-if="toastError" class="text-red-600 text-sm font-medium">{{ toastError }}</div>
        </div>
      </div>
      
      <!-- API Keys Management Section -->
      <div class="mt-8 card p-6">
        <h2 class="text-xl font-semibold mb-4">API Keys Management</h2>
        
        <div v-if="apiKeysLoading" class="text-center text-muted py-4">
          Loading API keys...
        </div>
        
        <div v-else>
          <!-- API Keys Table -->
          <div v-if="apiKeys.length > 0" class="mb-4">
            <div class="bg-gray-50 rounded-lg overflow-hidden">
              <table class="w-full">
                <thead class="bg-gray-100">
                  <tr>
                    <th class="px-4 py-3 text-left text-sm font-medium text-gray-700">API Key</th>
                    <th class="px-4 py-3 text-left text-sm font-medium text-gray-700">Actions</th>
                  </tr>
                </thead>
                <tbody>
                  <tr 
                    v-for="(apiKey, index) in apiKeys" 
                    :key="apiKey"
                    class="border-t border-gray-200"
                    :class="{ 'opacity-50': deletingApiKey === apiKey }"
                  >
                    <td class="px-4 py-3">
                      <div class="relative inline-block">
                        <code 
                          class="font-mono text-sm bg-gray-100 px-2 py-1 rounded hover:bg-green-100 hover:text-green-700 transition-colors cursor-pointer group inline-block"
                          @click="copyApiKey(apiKey)"
                          :title="copiedApiKey === apiKey ? 'Copied!' : 'Click to copy'"
                        >
                          {{ apiKey }}
                        </code>
                        <span 
                          v-if="copiedApiKey === apiKey"
                          class="absolute -top-8 left-1/2 transform -translate-x-1/2 bg-green-600 text-white text-xs px-2 py-1 rounded shadow-lg"
                        >
                          Copied!
                        </span>
                      </div>
                    </td>
                    <td class="px-4 py-3">
                      <button
                        @click="deleteApiKey(apiKey)"
                        :disabled="deletingApiKey === apiKey"
                        class="text-red-600 hover:text-red-800 disabled:opacity-50 disabled:cursor-not-allowed text-sm font-medium"
                      >
                        {{ deletingApiKey === apiKey ? 'Deleting...' : 'Delete' }}
                      </button>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
          
          <!-- No API Keys Message -->
          <div v-else class="text-center text-gray-500 py-8">
            <p class="mb-2">No API keys found.</p>
            <p class="text-sm">Create your first API key using the button below.</p>
          </div>
          
          <!-- Create API Key Button -->
          <div class="flex justify-center">
            <button 
              @click="createApiKey" 
              :disabled="creatingApiKey"
              class="inline-flex items-center px-3 py-2 border border-transparent text-sm leading-4 font-medium rounded-md text-white bg-green-600 hover:bg-green-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-green-500 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
            >
              <svg v-if="!creatingApiKey" class="w-4 h-4 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6"></path>
              </svg>
              <svg v-else class="animate-spin w-4 h-4 mr-1" fill="none" viewBox="0 0 24 24">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
              </svg>
              {{ creatingApiKey ? 'Creating...' : 'Create New API Key' }}
            </button>
          </div>
          
          <!-- Error Message -->
          <div v-if="apiKeysError" class="text-red-600 text-center mt-4">{{ apiKeysError }}</div>
        </div>
      </div>
      
      <div v-if="error" class="text-error mt-4">{{ error }}</div>
    </div>
  </div>
</template>

<style scoped>
.transition-colors {
  transition: color 0.2s ease-in-out, background-color 0.2s ease-in-out;
}

.transition-opacity {
  transition: opacity 0.2s ease-in-out;
}
</style>
