<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { apiService } from '@/services/api'
import { cookieUtils } from '@/utils/cookies'
import type { Competition, User, Team } from '@/types'
import { UserGroupIcon, TrophyIcon, ChartBarIcon } from '@heroicons/vue/24/outline'

const loading = ref(true)
const competition = ref<Competition>()
const user = ref<User>()
const team = ref<Team>()
const userInfo = ref<any>()
const error = ref('')

onMounted(async () => {
  try {
    userInfo.value = cookieUtils.getUserInfo()
    
    // Load competition data
    const [competitionData, userData, teamData] = await Promise.all([
      apiService.getCompetition(),
      apiService.getCurrentUser(),
      apiService.getUserTeam()
    ])
    
    competition.value = competitionData
    user.value = userData
    team.value = teamData
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
            Welcome to the cybersecurity competition platform
          </p>
          
          <div v-if="competition" class="grid grid-cols-1 md:grid-cols-3 gap-4 text-sm">
            <div>
              <span class="font-medium text-gray-700">Status:</span>
              <span class="ml-2 px-2 py-1 rounded-full text-xs font-medium"
                    :class="competition.status.Active ? 'bg-green-100 text-green-800' : 'bg-red-100 text-red-800'">
                {{ competition.status.Active ? 'Active' : 'Inactive' }}
              </span>
            </div>
            <div>
              <span class="font-medium text-gray-700">Start:</span>
              <span class="ml-2 text-gray-600">{{ competition.status.Active ? new Date(competition.status.Active.start_time) : "N/A" }}</span>
            </div>
            <div>
              <span class="font-medium text-gray-700">End:</span>
              <span class="ml-2 text-gray-600">{{ competition.status.Active ? competition.status.Active.end_time ? new Date(competition.status.Active.end_time) : "N/A" : "N/A" }}</span>
            </div>
          </div>
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
          </div>
        </div>
      </div>

      <!-- Quick Actions -->
      <div class="card p-6">
        <h2 class="text-xl font-semibold text-gray-900 mb-4">Quick Actions</h2>
        <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
          <RouterLink 
            to="/leaderboard" 
            class="flex items-center p-4 border border-gray-200 rounded-lg hover:border-primary-300 hover:bg-white transition-colors"
          >
            <TrophyIcon class="h-8 w-8 text-black mr-3" />
            <div>
              <div class="font-medium text-gray-900">Leaderboard</div>
              <div class="text-sm text-gray-600">View team rankings</div>
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
