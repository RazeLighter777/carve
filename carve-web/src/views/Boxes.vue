<script setup lang="ts">
import { ref, onMounted } from 'vue'
import apiService from '@/services/api'
import { useRouter } from 'vue-router'

const teams = ref<Array<{id: number, name: string}>>([])
const boxes = ref<Array<{name: string, ipAddress?: string, status?: string, loading?: boolean}>>([])
const selectedTeamName = ref<string>('')
const selectedTeamId = ref<string>('')
const userInfo = ref<any>(null)
const loading = ref(true)
const router = useRouter()

const fetchBoxesWithDetails = async (teamId: string) => {
  const boxList = await apiService.getBoxes(teamId)
  // Set initial boxes with loading placeholders
  boxes.value = boxList.map(box => ({ ...box, ipAddress: undefined, status: undefined, loading: true }))
  // For each box, fetch its details (ip and status) asynchronously
  boxList.forEach(async (box, idx) => {
    try {
      const res = await apiService.getBox(box.name)
      boxes.value[idx] = { ...box, ipAddress: res.ipAddress, status: res.status, loading: false }
    } catch (e) {
      boxes.value[idx] = { ...box, ipAddress: 'N/A', status: 'unknown', loading: false }
    }
  })
}

const fetchTeamsAndBoxes = async () => {
  loading.value = true
  teams.value = await apiService.getTeams()
  // Set default team only if not already set
  if (!selectedTeamName.value) {
    // Try to use userInfo.team_id if available and matches a team
    const userTeamName = userInfo.value?.team_name?.toString()
    selectedTeamName.value = userTeamName
    selectedTeamId.value = teams.value.find(t => t.name.toString() === userTeamName)?.id.toString() || ''
  }
  if (selectedTeamName.value) {
    selectedTeamId.value = teams.value.find(t => t.name.toString() === selectedTeamName.value)?.id.toString() || ''
    await fetchBoxesWithDetails(selectedTeamId.value)
  }
  loading.value = false
}

onMounted(async () => {
  userInfo.value = (await import('@/utils/cookies')).cookieUtils.getUserInfo()
  await fetchTeamsAndBoxes()
})

const onTeamChange = async () => {
  await fetchTeamsAndBoxes()
}
</script>

<template>
  <div class="max-w-3xl mx-auto px-4 sm:px-6 lg:px-8 py-8 bg-white dark:bg-gray-900 rounded-xl shadow-md">
    <h1 class="text-3xl font-bold mb-6 text-subheading dark:text-gray-100">Boxes</h1>
    <div class="mb-6 flex items-center space-x-4">
      <label for="team-select" class="text-body font-medium dark:text-gray-200">Team:</label>
      <select id="team-select" v-model="selectedTeamName" @change="onTeamChange" class="input-field w-64 dark:bg-gray-800 dark:text-gray-100 dark:border-gray-600 dark:placeholder-gray-400">
        <option v-for="team in teams" :key="team.id" :value="team.name" class="dark:bg-gray-800 dark:text-gray-100">{{ team.name }}</option>
      </select>
    </div>
    <div v-if="loading" class="text-muted dark:text-gray-400">Loading...</div>
    <div v-else>
      <div v-if="boxes.length === 0" class="text-muted dark:text-gray-400">No boxes found for this team.</div>
      <div v-else class="grid grid-cols-1 gap-4">
        <div class="card p-0 overflow-x-auto dark:bg-gray-800 dark:border-gray-700">
          <table class="min-w-full divide-y divide-gray-200 dark:divide-gray-700">
            <thead class="bg-gray-50 dark:bg-gray-900">
              <tr>
                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">Box Name</th>
                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">IP Address</th>
                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">Status</th>
                <th class="px-6 py-3"></th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="box in boxes" :key="box.name" class="hover:bg-gray-50 dark:hover:bg-gray-700">
                <td class="px-6 py-4 font-medium text-body dark:text-gray-100">{{ box.name }}</td>
                <td class="px-6 py-4">
                  <span v-if="box.loading" class="text-muted dark:text-gray-400 animate-pulse">Loading...</span>
                  <span v-else class="dark:text-gray-100">{{ box.ipAddress || 'N/A' }}</span>
                </td>
                <td class="px-6 py-4">
                  <span v-if="box.loading" class="badge badge-warning animate-pulse dark:bg-yellow-900 dark:text-yellow-200">Loading...</span>
                  <span v-else :class="{
                    'badge badge-success dark:bg-green-900 dark:text-green-200': box.status === 'active',
                    'badge badge-error dark:bg-red-900 dark:text-red-200': box.status === 'inactive' || box.status === 'unknown',
                    'badge badge-warning dark:bg-yellow-900 dark:text-yellow-200': !box.status || box.status === 'pending',
                  }">
                    {{ box.status || 'unknown' }}
                  </span>
                </td>
                <td class="px-6 py-4 text-right min-w-[160px]">
                  <template v-if="userInfo && userInfo.team_name && selectedTeamName && userInfo.team_name.toString() === selectedTeamName.toString()">
                    <router-link :to="`/console/${encodeURIComponent((teams.find(t => t.name.toString() === selectedTeamName.toString())?.name || ''))}/${encodeURIComponent(box.name.split('.')[0])}`" class="btn-secondary w-full block text-center dark:bg-blue-900 dark:text-blue-200 dark:border-blue-700">
                      Console
                    </router-link>
                  </template>
                  <template v-else>
                    <button class="btn-secondary w-full block text-center opacity-50 cursor-not-allowed dark:bg-blue-900 dark:text-blue-200 dark:border-blue-700" disabled>
                      Console
                    </button>
                  </template>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>
  </div>
</template>
