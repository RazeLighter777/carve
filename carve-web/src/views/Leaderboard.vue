<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { apiService } from '@/services/api'
import type { LeaderboardEntry } from '@/types'
import { TrophyIcon, ArrowPathIcon } from '@heroicons/vue/24/outline'

const loading = ref(true)
const leaderboard = ref<LeaderboardEntry[]>([])
const error = ref('')
const lastUpdated = ref<Date>()

const loadLeaderboard = async () => {
  try {
    loading.value = true
    error.value = ''
    
    const data = await apiService.getLeaderboard()
    leaderboard.value = data
    lastUpdated.value = new Date()
  } catch (err) {
    console.error('Failed to load leaderboard:', err)
    error.value = 'Failed to load leaderboard data'
  } finally {
    loading.value = false
  }
}

const refresh = () => {
  loadLeaderboard()
}

const getRankColor = (rank: number) => {
  switch (rank) {
    case 1:
      return 'text-yellow-600 bg-yellow-50'
    case 2:
      return 'text-gray-600 bg-gray-50'
    case 3:
      return 'text-orange-600 bg-orange-50'
    default:
      return 'text-gray-700 bg-gray-100'
  }
}

const getRankIcon = (rank: number) => {
  return rank <= 3 ? '🏆' : `#${rank}`
}

onMounted(() => {
  loadLeaderboard()
})
</script>

<template>
  <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
    <div class="mb-8">
      <div class="flex items-center justify-between">
        <div class="flex items-center">
          <TrophyIcon class="h-8 w-8 text-black mr-3" />
          <div>
            <h1 class="text-3xl font-bold text-gray-900">Leaderboard</h1>
            <p class="text-gray-600 mt-1">Current team rankings and scores</p>
          </div>
        </div>
        
        <button
          @click="refresh"
          :disabled="loading"
          class="btn-secondary flex items-center"
          :class="{ 'opacity-50 cursor-not-allowed': loading }"
        >
          <ArrowPathIcon class="h-4 w-4 mr-2" :class="{ 'animate-spin': loading }" />
          Refresh
        </button>
      </div>
      
      <div v-if="lastUpdated" class="text-sm text-gray-500 mt-2">
        Last updated: {{ lastUpdated.toLocaleTimeString() }}
      </div>
    </div>

    <!-- Loading state -->
    <div v-if="loading && !leaderboard.length" class="flex justify-center items-center min-h-96">
      <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-gray-300"></div>
    </div>

    <!-- Error state -->
    <div v-else-if="error" class="card p-6 text-center">
      <p class="text-red-600">{{ error }}</p>
      <button @click="refresh" class="btn-primary mt-4">Try Again</button>
    </div>

    <!-- Leaderboard content -->
    <div v-else-if="leaderboard.length" class="card overflow-hidden">
      <div class="overflow-x-auto">
        <table class="min-w-full divide-y divide-gray-200">
          <thead class="bg-gray-50">
            <tr>
              <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                Rank
              </th>
              <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                Team
              </th>
              <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                Score
              </th>
              <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">
                Team ID
              </th>
            </tr>
          </thead>
          <tbody class="bg-white divide-y divide-gray-200">
            <tr v-for="entry in leaderboard" :key="entry.teamId" 
                class="hover:bg-gray-50 transition-colors">
              <td class="px-6 py-4 whitespace-nowrap">
                <div class="flex items-center">
                  <span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium"
                        :class="getRankColor(entry.rank)">
                    {{ getRankIcon(entry.rank) }}
                  </span>
                </div>
              </td>
              <td class="px-6 py-4 whitespace-nowrap">
                <div class="text-sm font-medium text-gray-900">
                  {{ entry.teamName }}
                </div>
              </td>
              <td class="px-6 py-4 whitespace-nowrap">
                <div class="text-sm text-gray-900 font-mono">
                  {{ entry.score.toLocaleString() }}
                </div>
              </td>
              <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">
                {{ entry.teamId }}
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- Empty state -->
    <div v-else class="card p-12 text-center">
      <TrophyIcon class="h-12 w-12 text-gray-400 mx-auto mb-4" />
      <h3 class="text-lg font-medium text-gray-900 mb-2">No Results Yet</h3>
      <p class="text-gray-600">The competition hasn't started or no scores have been recorded yet.</p>
    </div>
  </div>
</template>
