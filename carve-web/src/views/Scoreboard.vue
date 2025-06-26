<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { apiService } from '@/services/api'
import { type CheckResponse, type TeamCheckStatusResponse, type ScoreboardEntry, type Team } from '@/types'
import { ChartBarIcon, FunnelIcon, ArrowPathIcon, TrophyIcon } from '@heroicons/vue/24/outline'
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
const checks = ref<CheckResponse>({ checks: [], flag_checks: [] })
const selectedTeamCheckStatus = ref<TeamCheckStatusResponse>({  checks: [], flag_checks: [] })
const allTeamsCheckStatus = ref<Record<number, TeamCheckStatusResponse>>({})
const error = ref('')
const lastUpdated = ref<Date>()

// Filters
const selectedTeam = ref<string>('')
const selectedTeamId = computed(() => {
  return selectedTeam.value ? Number(selectedTeam.value) : null
})
const selectedCheck = ref('')

const timeOptions = [
  { label: '5 minutes', value: 5 * 60 },
  { label: '20 minutes', value: 20 * 60 },
  { label: '1 hour', value: 60 * 60 },
  { label: '8 hours', value: 8 * 60 * 60 },
]
const selectedTime = ref(timeOptions[0].value)

const allCheckNames = computed(() => {
  // Combine both checks and flag_checks for dropdown
  return [
    ...checks.value.checks.map(c => ({ name: c.name, type: 'check' })),
    ...checks.value.flag_checks.map(f => ({ name: f.name, type: 'flag' }))
  ]
})

const filteredScoreboard = computed(() => {
  let filtered = scoreboard.value

  if (selectedTeam.value) {
    filtered = filtered.filter(entry => entry.team_id === selectedTeamId.value)
  }

  if (selectedCheck.value) {
    filtered = filtered.filter(entry => entry.score_event_type === selectedCheck.value)
  }

  // Sort by timestamp (newest first)
  return filtered.sort((a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime())
})

const getStartTime = () => {
  // Return a Date object for UTC now minus selectedTime (in seconds)
  const now = new Date()
  return new Date(now.getTime() - selectedTime.value * 1000)
}

const loadData = async () => {
  try {
    loading.value = true
    error.value = ''

    const [scoreboardData, teamsData, checksData] = await Promise.all([
      apiService.getScoreboard(selectedTeam.value, selectedCheck.value, getStartTime()),
      apiService.getTeams(),
      apiService.getChecks(),
    ])

    scoreboard.value = scoreboardData
    teams.value = teamsData
    checks.value = checksData

    if (selectedTeam.value && selectedTeamId.value !== null) {
      // Only fetch for selected team
      const teamCheckStatusData = await apiService.getCheckStatus(selectedTeamId.value)
      selectedTeamCheckStatus.value = teamCheckStatusData
      allTeamsCheckStatus.value = { [selectedTeamId.value]: teamCheckStatusData }
    } else {
      // Fetch for all teams
      const statusResults = await Promise.all(
        teamsData.map(team => apiService.getCheckStatus(team.id).then(
          res => [team.id, res] as [number, TeamCheckStatusResponse]
        ))
      )
      allTeamsCheckStatus.value = Object.fromEntries(statusResults)
      selectedTeamCheckStatus.value = { checks: [], flag_checks: [] }
    }
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
  apiService.getTeam(teamId).then(team => {
    return team ? team.name : 'Unknown Team'
  }).catch(() => {
    return 'Unknown Team'
  })
}

const checkPointsMap = computed(() => {
  const map = new Map<string, number>()
  checks.value.checks.forEach(check => {
    map.set(check.name, check.points)
  })
  return map
})

const lineData = computed(() => {
  // Group by team, accumulate points over time
  const teamMap = new Map<number, { label: string, data: Array<{ x: string, y: number }> }>()
  // Get all unique timestamps sorted
  const allTimestamps = Array.from(new Set(scoreboard.value.map(e => e.timestamp))).sort()
  let teamsToShow = teams.value
  if (selectedTeam.value && selectedTeamId.value !== null) {
    teamsToShow = teams.value.filter(team => team.id === selectedTeamId.value)
  }
  teamsToShow.forEach(team => {
    let total = 0
    const pointsByTime: Array<{ x: string, y: number }> = []
    allTimestamps.forEach(ts => {
      // Sum all points for this team up to this timestamp
      const events = scoreboard.value.filter(e => e.team_id === team.id && e.timestamp <= ts).filter(e => e.score_event_type === selectedCheck.value || !selectedCheck.value)
      total = events.reduce((sum, e) => sum + (checkPointsMap.value.get(e.score_event_type) || 0), 0)
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
      tension: 0.2,
      pointRadius: 0 // Hide points on the graph
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

// Add filtered computed properties for check status
const filteredChecks = computed(() => {
  if (selectedTeam.value) {
    return selectedTeamCheckStatus.value.checks
      .filter(check => !selectedCheck.value || check.name === selectedCheck.value)
      .map(check => ({ ...check, team_id: selectedTeamId.value }))
  } else {
    const result: Array<any> = []
    Object.entries(allTeamsCheckStatus.value).forEach(([teamId, status]) => {
      status.checks
        .filter(check => !selectedCheck.value || check.name === selectedCheck.value)
        .forEach(check => {
          result.push({ ...check, team_id: Number(teamId) })
        })
    })
    return result
  }
})
const filteredFlagChecks = computed(() => {
  if (selectedTeam.value) {
    return selectedTeamCheckStatus.value.flag_checks
      .filter(flag => !selectedCheck.value || flag.name === selectedCheck.value)
      .map(flag => ({ ...flag, team_id: selectedTeamId.value }))
  } else {
    const result: Array<any> = []
    Object.entries(allTeamsCheckStatus.value).forEach(([teamId, status]) => {
      status.flag_checks
        .filter(flag => !selectedCheck.value || flag.name === selectedCheck.value)
        .forEach(flag => {
          result.push({ ...flag, team_id: Number(teamId) })
        })
    })
    return result
  }
})
const getTeamNameById = (id: number) => {
  const team = teams.value.find(t => t.id === id)
  return team ? team.name : 'Unknown Team'
}

const selectedTeamName = computed(() => {
  if (!selectedTeam.value) return 'All Teams'
  const team = teams.value.find(t => t.id.toString() === selectedTeam.value)
  return team ? team.name : 'Unknown Team'
})

// Leaderboard logic
import type { LeaderboardEntry } from '@/types'
const leaderboard = ref<LeaderboardEntry[]>([])
const leaderboardLoading = ref(true)
const leaderboardError = ref('')
const leaderboardLastUpdated = ref<Date>()
const expandedTeamId = ref<string | null>(null)
const expandedTeamMembers = ref<Record<string, Team['members']>>({})
const expandedTeamLoading = ref<Record<string, boolean>>({})
const expandedTeamError = ref<Record<string, string>>({})

const loadLeaderboard = async () => {
  try {
    leaderboardLoading.value = true
    leaderboardError.value = ''
    const data = await apiService.getLeaderboard()
    leaderboard.value = data
    leaderboardLastUpdated.value = new Date()
  } catch (err) {
    leaderboardError.value = 'Failed to load leaderboard data'
  } finally {
    leaderboardLoading.value = false
  }
}

const leaderboardRefresh = () => {
  loadLeaderboard()
}

const toggleExpand = async (teamId: string) => {
  if (expandedTeamId.value === teamId) {
    expandedTeamId.value = null
    return
  }
  expandedTeamId.value = teamId
  if (!expandedTeamMembers.value[teamId]) {
    expandedTeamLoading.value[teamId] = true
    expandedTeamError.value[teamId] = ''
    try {
      const team = await apiService.getTeam(Number(teamId))
      expandedTeamMembers.value[teamId] = team.members
    } catch (err: any) {
      expandedTeamError.value[teamId] = 'Failed to load team members'
    } finally {
      expandedTeamLoading.value[teamId] = false
    }
  }
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
  loadData()
  loadLeaderboard()
})
</script>

<template>
  <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
    <!-- Leaderboard Section -->
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
          @click="leaderboardRefresh"
          :disabled="leaderboardLoading"
          class="btn-secondary flex items-center"
          :class="{ 'opacity-50 cursor-not-allowed': leaderboardLoading }"
        >
          <ArrowPathIcon class="h-4 w-4 mr-2" :class="{ 'animate-spin': leaderboardLoading }" />
          Refresh
        </button>
      </div>
      <div v-if="leaderboardLastUpdated" class="text-sm text-gray-500 mt-2">
        Last updated: {{ leaderboardLastUpdated.toLocaleTimeString() }}
      </div>
    </div>
    <!-- Leaderboard Table -->
    <div v-if="leaderboardLoading && !leaderboard.length" class="flex justify-center items-center min-h-96">
      <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-gray-300"></div>
    </div>
    <div v-else-if="leaderboardError" class="card p-6 text-center">
      <p class="text-red-600">{{ leaderboardError }}</p>
      <button @click="leaderboardRefresh" class="btn-primary mt-4">Try Again</button>
    </div>
    <div v-else-if="leaderboard.length" class="card overflow-hidden mb-8">
      <div class="overflow-x-auto">
        <table class="min-w-full divide-y divide-gray-200">
          <thead class="bg-gray-50">
            <tr>
              <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Rank</th>
              <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Team</th>
              <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Score</th>
              <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Team ID</th>
            </tr>
          </thead>
          <tbody class="bg-white divide-y divide-gray-200">
            <template v-for="entry in leaderboard" :key="entry.teamId">
              <tr
                class="hover:bg-gray-50 transition-colors cursor-pointer"
                @click="toggleExpand(entry.teamId.toString())"
                :class="{ 'bg-gray-100': expandedTeamId === entry.teamId.toString() }"
              >
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
              <tr v-if="expandedTeamId === entry.teamId.toString()">
                <td colspan="4" class="bg-gray-50 px-8 py-4">
                  <div>
                    <h4 class="font-semibold text-gray-700 mb-2">Team Members</h4>
                    <div v-if="expandedTeamLoading[entry.teamId.toString()]" class="text-gray-500">Loading...</div>
                    <div v-else-if="expandedTeamError[entry.teamId.toString()]" class="text-red-600">{{ expandedTeamError[entry.teamId.toString()] }}</div>
                    <table v-else class="min-w-full text-sm">
                      <thead>
                        <tr class="bg-gray-100">
                          <th class="px-4 py-2 text-left">Name</th>
                        </tr>
                      </thead>
                      <tbody>
                        <tr v-for="member in expandedTeamMembers[entry.teamId.toString()] || []" :key="member.name">
                          <td class="px-4 py-2">{{ member.name || 'Unknown' }}</td>
                        </tr>
                        <tr v-if="!expandedTeamMembers[entry.teamId.toString()] || !expandedTeamMembers[entry.teamId.toString()].length">
                          <td class="px-4 py-2 text-gray-400">No members found.</td>
                        </tr>
                      </tbody>
                    </table>
                  </div>
                </td>
              </tr>
            </template>
          </tbody>
        </table>
      </div>
    </div>
    <div v-else class="card p-12 text-center mb-8">
      <TrophyIcon class="h-12 w-12 text-gray-400 mx-auto mb-4" />
      <h3 class="text-lg font-medium text-gray-900 mb-2">No Results Yet</h3>
      <p class="text-gray-600">The competition hasn't started or no scores have been recorded yet.</p>
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
            <option v-for="check in allCheckNames" :key="check.name" :value="check.name">
              {{ check.name }} <span v-if="check.type === 'flag'">[Flag]</span>
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

    <!-- Time Range Dropdown -->
    <div class="mb-4 flex items-center">
      <label class="block text-sm font-medium text-gray-700 mr-2">Time Range:</label>
      <select v-model="selectedTime" @change="loadData" class="input-field w-auto">
        <option v-for="option in timeOptions" :key="option.value" :value="option.value">
          {{ option.label }}
        </option>
      </select>
    </div>
    <!-- Scoreboard content -->
    <div v-if="filteredScoreboard.length" class="space-y-4">
      <div class="text-sm text-gray-600 mb-4">
        Showing {{ filteredScoreboard.length }} event{{ filteredScoreboard.length !== 1 ? 's' : '' }}
      </div>
      <div class="bg-white rounded shadow p-4 mb-6">
        <Line :data="lineData" :options="lineOptions" class="h-60" />
      </div>
    </div>

    <!-- Check Status Section -->
    <div v-if="filteredChecks.length || filteredFlagChecks.length" class="mt-8">
      <div class="mb-4">
        <h2 class="text-xl font-bold text-gray-900">Current Check Status</h2>
        <p class="text-gray-600 text-sm">Status for selected team and check</p>
      </div>
      <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
        <!-- Status Checks -->
        <div>
          <h3 class="text-lg font-semibold mb-2">Checks</h3>
          <div v-if="filteredChecks.length">
            <table class="min-w-full divide-y divide-gray-200">
              <thead>
                <tr>
                  <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">Team</th>
                  <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">Name</th>
                  <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">Status</th>
                  <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">Message</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="check in filteredChecks" :key="check.name + '-' + check.team_id">
                  <td class="px-4 py-2">{{ getTeamNameById(check.team_id) }}</td>
                  <td class="px-4 py-2">{{ check.name }}</td>
                  <td class="px-4 py-2">
                    <span v-if="check.failed_for === 0 && !check.passing" class="text-gray-500 font-bold">Pending</span>
                    <span v-else :class="check.passing ? 'text-green-600 font-bold' : 'text-red-600 font-bold'">
                      {{ check.passing ? 'Passing' : `Failing for ${check.failed_for} checks now` }}
                    </span>
                  </td>
                  <td class="px-4 py-2">{{ check.message }}</td>
                </tr>
              </tbody>
            </table>
          </div>
          <div v-else class="text-gray-500">No checks available.</div>
        </div>
        <!-- Flag Checks -->
        <div>
          <h3 class="text-lg font-semibold mb-2">Flag Checks</h3>
          <div v-if="filteredFlagChecks.length">
            <table class="min-w-full divide-y divide-gray-200">
              <thead>
                <tr>
                  <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">Team</th>
                  <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">Name</th>
                  <th class="px-4 py-2 text-left text-xs font-medium text-gray-500 uppercase">Status</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="flag in filteredFlagChecks" :key="flag.name + '-' + flag.team_id">
                  <td class="px-4 py-2">{{ getTeamNameById(flag.team_id) }}</td>
                  <td class="px-4 py-2">{{ flag.name }}</td>
                  <td class="px-4 py-2">
                    <span :class="flag.passing ? 'text-green-600 font-bold' : 'text-gray-500 font-bold'" v-if="flag.passing">Found</span>
                    <span :class="flag.passing ? 'text-green-600 font-bold' : 'text-red-600 font-bold'" v-else>Missing</span>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
          <div v-else class="text-gray-500">No flag checks available.</div>
        </div>
      </div>
    </div>
  </div>
</template>
