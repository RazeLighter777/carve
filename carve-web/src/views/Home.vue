<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { apiService } from '@/services/api'
import { cookieUtils } from '@/utils/cookies'
import type { CompetitionState, User, Team } from '@/types'
import { UserGroupIcon, CubeTransparentIcon, ChartBarIcon } from '@heroicons/vue/24/outline'
import CompetitionStatus from '@/components/CompetitionStatus.vue'

const loading = ref(true)
const competition = ref<any>(null)
const user = ref<User>()
const team = ref<Team>()
const userInfo = ref<any>()
const error = ref('')

onMounted(async () => {
  try {
    userInfo.value = cookieUtils.getUserInfo()
    
    // Load competition data
    competition.value = await apiService.getCompetition()
    user.value = await apiService.getCurrentUser()
    team.value = await apiService.getUserTeam()
  } catch (err) {
    console.error('Failed to load home data:', err)
    error.value = 'Failed to load competition data'
  } finally {
    loading.value = false
  }
})

const formatDate = (dateString: string) => {
  return new Date(dateString).toLocaleDateString('en-US', {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit'
  })
}

const joinCode = ref<string | null>(null)
const joinCodeError = ref('')
const joinCodeLoading = ref(false)
const joinCodeSuccess = ref(false)
const switchCode = ref('')
const switchCodeError = ref('')
const switchCodeLoading = ref(false)

const generateJoinCode = async () => {
  joinCodeError.value = ''
  joinCodeSuccess.value = false
  joinCodeLoading.value = true
  try {
    const result = await apiService.generateJoinCode()
    joinCode.value = result.code
    joinCodeSuccess.value = true
  } catch (err: any) {
    joinCodeError.value = err?.response?.data?.error || 'Failed to generate join code.'
  } finally {
    joinCodeLoading.value = false
  }
}

const switchTeam = async () => {
  switchCodeError.value = ''
  switchCodeLoading.value = true
  try {
    await apiService.switchTeam(switchCode.value.trim())
    window.location.reload()
  } catch (err: any) {
    switchCodeError.value = err?.response?.data?.error || 'Failed to join team. Please check your code.'
  } finally {
    switchCodeLoading.value = false
  }
}
</script>

<template>
  <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
    <!-- Loading state -->
    <div v-if="loading" class="flex justify-center items-center min-h-96">
      <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-gray-300"></div>
    </div>

    <!-- Error state -->
    <div v-else-if="error" class="text-center py-12">
      <p class="text-red-600">{{ error }}</p>
    </div>

    <!-- Main content -->
    <div v-else class="space-y-8">
      <!-- Competition Info -->
      <div class="card p-6">
        <div class="text-center">
          <h1 class="text-3xl font-bold text-gray-900 mb-2">
            {{ competition?.name || 'CARVE Competition' }}
          </h1>
          <p class="text-lg text-gray-600 mb-6">
            Welcome to {{ competition?.name || 'CARVE Competition' }}!
          </p>

          <!-- Game Explanation -->
          <div class="mb-6 text-base text-gray-700 text-left mx-auto">
            <p class="mb-2">
              This is a Capture The Flag (CTF) competition where you can test your skills in cybersecurity.
              Hack into other teams' boxes, defend your own, and find hidden flags to score points.
            </p>
            <p class="mb-2">
              Once the competition starts, you can access your private network and servers for the game on the <RouterLink to="/boxes" class="text-primary-600 underline">/boxes</RouterLink> page.
            </p>
            <p class="mb-2">
              Score points by keeping your services online and by locating hidden flags on your boxes. You can submit flags and see your service status at the <RouterLink to="/compete" class="text-primary-600 underline">/compete</RouterLink> page.
            </p>
            <p class="mb-2">
              Each team receives a <span class="font-mono">/24</span> subnet with identical machines. You can find other teams' boxes using DNS records in the format <span class="font-mono">&lt;box name&gt;.&lt;team name&gt;.{{ (competition?.name || 'carve').toLowerCase() }}.hack.</span> Or you can get their IP addresses directly by visiting the boxes page and selecting the team in the drop down menu.
            </p>
          </div>

          <CompetitionStatus :competition="competition"/>
        </div>
      </div>

      <!-- User and Team Info -->
      <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <!-- User Info -->
        <div class="card p-6">
          <div class="flex items-center mb-4">
            <UserGroupIcon class="h-6 w-6 text-black mr-2" />
            <h2 class="text-xl font-semibold text-gray-900">Your Profile</h2>
          </div>
          
          <div v-if="user && userInfo" class="space-y-3">
            <div>
              <span class="font-medium text-gray-700">Name:</span>
              <span class="ml-2 text-gray-900">{{ user.name }}</span>
            </div>
            <div>
              <span class="font-medium text-gray-700">Email:</span>
              <span class="ml-2 text-gray-600">{{ user.email }}</span>
            </div>
            <div>
              <span class="font-medium text-gray-700">Username:</span>
              <span class="ml-2 text-gray-600">{{ userInfo.username }}</span>
            </div>
            <div v-if="userInfo.is_admin">
              <span class="px-2 py-1 bg-yellow-100 text-yellow-800 text-xs font-medium rounded-full">
                Administrator
              </span>
            </div>
          </div>
        </div>

        <!-- Team Info -->
        <div class="card p-6">
          <div class="flex items-center mb-4">
            <TrophyIcon class="h-6 w-6 text-black mr-2" />
            <h2 class="text-xl font-semibold text-gray-900">Your Team</h2>
          </div>
          <div v-if="team" class="space-y-4">
            <div>
              <span class="font-medium text-gray-700">Team Name:</span>
              <span class="ml-2 text-gray-900 font-medium">{{ team.name }}</span>
            </div>
            <div>
              <span class="font-medium text-gray-700 block mb-2">Team Members:</span>
              <div class="space-y-2">
                <div v-for="member in team.members" :key="member.name" 
                     class="flex items-center p-2 bg-gray-50 rounded-md">
                  <div class="w-8 h-8 bg-primary-100 rounded-full flex items-center justify-center mr-3">
                    <span class="text-black font-medium text-sm">
                      {{ member.name.charAt(0).toUpperCase() }}
                    </span>
                  </div>
                  <span class="text-gray-900">{{ member.name }}</span>
                </div>
              </div>
            </div>
            <div class="mt-6 space-y-4">
              <button @click="generateJoinCode" class="btn-primary w-full" :disabled="joinCodeLoading">
                {{ joinCodeLoading ? 'Generating...' : 'Generate Team Join Code' }}
              </button>
              <div v-if="joinCode" class="mt-2 text-center">
                <span class="font-mono text-lg bg-gray-100 px-3 py-1 rounded">{{ joinCode }}</span>
                <span v-if="joinCodeSuccess" class="ml-2 text-green-600">Copied!</span>
              </div>
              <div v-if="joinCodeError" class="text-red-600 text-center mt-2">{{ joinCodeError }}</div>
              <form @submit.prevent="switchTeam" class="mt-6">
                <label class="block font-medium text-gray-700 mb-1">Switch Teams</label>
                <div class="flex space-x-2">
                  <input v-model="switchCode" type="text" inputmode="numeric" pattern="[0-9]*" maxlength="9" class="flex-1 px-3 py-2 border rounded focus:outline-none focus:ring-2 focus:ring-primary" placeholder="Enter team code" :disabled="switchCodeLoading" />
                  <button type="submit" class="btn-secondary" :disabled="switchCodeLoading">
                    {{ switchCodeLoading ? 'Joining...' : 'Join' }}
                  </button>
                </div>
                <div v-if="switchCodeError" class="text-red-600 mt-2">{{ switchCodeError }}</div>
              </form>
            </div>
          </div>
        </div>
      </div>

      <!-- Quick Actions -->
      <div class="card p-6">
        <h2 class="text-xl font-semibold text-gray-900 mb-4">Quick Actions</h2>
        <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
          <RouterLink 
            to="/boxes" 
            class="flex items-center p-4 border border-gray-200 rounded-lg hover:border-primary-300 hover:bg-white transition-colors"
          >
            <CubeTransparentIcon class="h-8 w-8 text-black mr-3" />
            <div>
              <div class="font-medium text-gray-900">Boxes</div>
              <div class="text-sm text-gray-600">Log into your boxes and hack the other teams!</div>
            </div>
          </RouterLink>
          
          <RouterLink 
            to="/scoreboard" 
            class="flex items-center p-4 border border-gray-200 rounded-lg hover:border-primary-300 hover:bg-white transition-colors"
          >
            <ChartBarIcon class="h-8 w-8 text-black mr-3" />
            <div>
              <div class="font-medium text-gray-900">Scoreboard</div>
              <div class="text-sm text-gray-600">View score history</div>
            </div>
          </RouterLink>
          
          <RouterLink 
            to="/about" 
            class="flex items-center p-4 border border-gray-200 rounded-lg hover:border-primary-300 hover:bg-white transition-colors"
          >
            <svg class="h-8 w-8 text-black mr-3" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
            </svg>
            <div>
              <div class="font-medium text-gray-900">About</div>
              <div class="text-sm text-gray-600">Platform information</div>
            </div>
          </RouterLink>
        </div>
      </div>
    </div>
  </div>
</template>
