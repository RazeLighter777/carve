<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { apiService } from '@/services/api'
import type { ScoreboardEntry, Team } from '@/types'
import { ChartBarIcon, FunnelIcon, ArrowPathIcon } from '@heroicons/vue/24/outline'
import { Line } from 'vue-chartjs'
import {
  Chart as ChartJS,
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  Title,
  Tooltip,
  Legend
} from 'chart.js'

ChartJS.register(
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  Title,
  Tooltip,
  Legend
)

const loading = ref(true)
const scoreboard = ref<ScoreboardEntry[]>([])
const teams = ref<Team[]>([])
const checks = ref<Array<{name: string, points: number}>>([])
const error = ref('')
const lastUpdated = ref<Date>()

// Filters
const selectedTeam = ref('')
const selectedCheck = ref('')

const filteredScoreboard = computed(() => {
  let filtered = scoreboard.value
  
  if (selectedTeam.value) {
    filtered = filtered.filter(entry => entry.teamId.toString() === selectedTeam.value)
  }
  
  if (selectedCheck.value) {
    filtered = filtered.filter(entry => entry.scoringCheck === selectedCheck.value)
  }
  
  // Sort by timestamp (newest first)
  return filtered.sort((a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime())
})

const loadData = async () => {
  try {
    loading.value = true
    error.value = ''
    
    const [scoreboardData, teamsData, checksData] = await Promise.all([
      apiService.getScoreboard(selectedTeam.value, selectedCheck.value),
      apiService.getTeams(),
      apiService.getChecks()
    ])
    
    scoreboard.value = scoreboardData
    teams.value = teamsData
    checks.value = checksData
    lastUpdated.value = new Date()
  } catch (err) {
    console.error('Failed to load scoreboard:', err)
    error.value = 'Failed to load scoreboard data'
  } finally {
    loading.value = false
  }
}

const refresh = () => {
  loadData()
}

const clearFilters = () => {
  selectedTeam.value = ''
  selectedCheck.value = ''
  loadData()
}

const formatTimestamp = (timestamp: string) => {
  return new Date(timestamp).toLocaleString('en-US', {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit'
  })
}

const getTeamName = (teamId: number) => {
  const team = teams.value.find(t => t.id === teamId)
  return team?.name || `Team ${teamId}`
}

const checkPointsMap = computed(() => {
  const map = new Map<string, number>()
  checks.value.forEach(check => {
    map.set(check.name, check.points)
  })
  return map
})

const lineData = computed(() => {
  // Group by team, accumulate points over time
  const teamMap = new Map<number, { label: string, data: Array<{ x: string, y: number }> }>()
  // Get all unique timestamps sorted
  const allTimestamps = Array.from(new Set(scoreboard.value.map(e => e.timestamp))).sort()
  teams.value.forEach(team => {
    let total = 0
    const pointsByTime: Array<{ x: string, y: number }> = []
    allTimestamps.forEach(ts => {
      // Sum all points for this team up to this timestamp
      const events = scoreboard.value.filter(e => e.teamId === team.id && e.timestamp <= ts)
      total = events.reduce((sum, e) => sum + (checkPointsMap.value.get(e.scoringCheck) || 0), 0)
      pointsByTime.push({ x: ts, y: total })
    })
    teamMap.set(team.id, { label: team.name, data: pointsByTime })
  })
  return {
    labels: allTimestamps.map(ts => new Date(ts).toLocaleTimeString()),
    datasets: Array.from(teamMap.values()).map((series, idx) => ({
      label: series.label,
      data: series.data.map(d => d.y),
      fill: false,
      borderColor: `hsl(${(idx * 60) % 360}, 70%, 50%)`,
      tension: 0.2
    }))
  }
})

const lineOptions = {
  responsive: true,
  plugins: {
    legend: { position: 'top' as const },
    title: { display: true, text: 'Score Progression Over Time' }
  },
  scales: {
    x: { title: { display: true, text: 'Time' } },
    y: { title: { display: true, text: 'Points' }, beginAtZero: true }
  }
}

onMounted(() => {
  loadData()
})
</script>

<template>
  <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
    <div class="mb-8">
      <div class="flex items-center justify-between">
        <div class="flex items-center">
          <ChartBarIcon class="h-8 w-8 text-black mr-3" />
          <div>
            <h1 class="text-3xl font-bold text-gray-900">Scoreboard</h1>
            <p class="text-gray-600 mt-1">Real-time scoring events and history</p>
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

    <!-- Filters -->
    <div class="card p-6 mb-6">
      <div class="flex items-center mb-4">
        <FunnelIcon class="h-5 w-5 text-gray-600 mr-2" />
        <h2 class="text-lg font-medium text-gray-900">Filters</h2>
      </div>
      
      <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
        <div>
          <label class="block text-sm font-medium text-gray-700 mb-2">Team</label>
          <select v-model="selectedTeam" @change="loadData" class="input-field">
            <option value="">All Teams</option>
            <option v-for="team in teams" :key="team.id" :value="team.id.toString()">
              {{ team.name }}
            </option>
          </select>
        </div>
        
        <div>
          <label class="block text-sm font-medium text-gray-700 mb-2">Scoring Check</label>
          <select v-model="selectedCheck" @change="loadData" class="input-field">
            <option value="">All Checks</option>
            <option v-for="check in checks" :key="check.name" :value="check.name">
              {{ check.name }} ({{ check.points }} pts)
            </option>
          </select>
        </div>
        
        <div class="flex items-end">
          <button @click="clearFilters" class="btn-secondary w-full">
            Clear Filters
          </button>
        </div>
      </div>
    </div>

    <!-- Loading state -->
    <div v-if="loading && !scoreboard.length" class="flex justify-center items-center min-h-96">
      <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-gray-300"></div>
    </div>

    <!-- Error state -->
    <div v-else-if="error" class="card p-6 text-center">
      <p class="text-red-600">{{ error }}</p>
      <button @click="refresh" class="btn-primary mt-4">Try Again</button>
    </div>

    <!-- Scoreboard content -->
    <div v-if="filteredScoreboard.length" class="space-y-4">
      <div class="text-sm text-gray-600 mb-4">
        Showing {{ filteredScoreboard.length }} event{{ filteredScoreboard.length !== 1 ? 's' : '' }}
      </div>
      <div class="bg-white rounded shadow p-4 mb-6">
        <Line :data="lineData" :options="lineOptions" class="h-60" />
      </div>
      <div class="space-y-3">
        <div v-for="entry in filteredScoreboard" :key="entry.id" 
             class="card p-4 hover:shadow-md transition-shadow">
          <div class="flex items-start justify-between">
            <div class="flex-1">
              <div class="flex items-center space-x-3 mb-2">
                <span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-primary-100 text-primary-800">
                  {{ getTeamName(entry.teamId) }}
                </span>
                <span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-800">
                  {{ entry.scoringCheck }}
                </span>
                <span class="text-xs text-gray-500">
                  {{ formatTimestamp(entry.timestamp) }}
                </span>
              </div>
              
              <div class="text-sm text-gray-900">
                {{ entry.message }}
              </div>
            </div>
            
            <div class="text-right">
              <div class="text-xs text-gray-500">Event ID</div>
              <div class="text-sm font-mono text-gray-700">{{ entry.id }}</div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Empty state -->
    <div v-else class="card p-12 text-center">
      <ChartBarIcon class="h-12 w-12 text-gray-400 mx-auto mb-4" />
      <h3 class="text-lg font-medium text-gray-900 mb-2">No Events Found</h3>
      <p class="text-gray-600">
        {{ selectedTeam || selectedCheck ? 'No events match your current filters.' : 'No scoring events have been recorded yet.' }}
      </p>
    </div>
  </div>
</template>
