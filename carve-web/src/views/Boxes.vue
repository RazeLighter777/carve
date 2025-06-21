<script setup lang="ts">
import { ref, onMounted } from 'vue'
import apiService from '@/services/api'
import { useRouter } from 'vue-router'

const teams = ref<Array<{id: number, name: string}>>([])
const boxes = ref<Array<{name: string, ipAddress?: string, status?: string, loading?: boolean}>>([])
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
  if (!selectedTeamId.value) {
    // Try to use userInfo.team_id if available and matches a team
    const userTeamId = userInfo.value?.team_id?.toString()
    if (userTeamId && teams.value.some(t => t.id.toString() === userTeamId)) {
      selectedTeamId.value = userTeamId
    } else if (teams.value.length > 0) {
      selectedTeamId.value = teams.value[0].id.toString()
    }
  }
  if (selectedTeamId.value) {
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
  <div class="max-w-3xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
    <h1 class="text-3xl font-bold mb-6 text-subheading">Boxes</h1>
    <div class="mb-6 flex items-center space-x-4">
      <label for="team-select" class="text-body font-medium">Team:</label>
      <select id="team-select" v-model="selectedTeamId" @change="onTeamChange" class="input-field w-64">
        <option v-for="team in teams" :key="team.id" :value="team.id">{{ team.name }}</option>
      </select>
    </div>
    <div v-if="loading" class="text-muted">Loading...</div>
    <div v-else>
      <div v-if="boxes.length === 0" class="text-muted">No boxes found for this team.</div>
      <div v-else class="grid grid-cols-1 gap-4">
        <div class="card p-0 overflow-x-auto">
          <table class="min-w-full divide-y divide-gray-200">
            <thead>
              <tr>
                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Box Name</th>
                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">IP Address</th>
                <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">Status</th>
                <th class="px-6 py-3"></th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="box in boxes" :key="box.name" class="hover:bg-gray-50">
                <td class="px-6 py-4 font-medium text-body">{{ box.name }}</td>
                <td class="px-6 py-4">
                  <span v-if="box.loading" class="text-muted animate-pulse">Loading...</span>
                  <span v-else>{{ box.ipAddress || 'N/A' }}</span>
                </td>
                <td class="px-6 py-4">
                  <span v-if="box.loading" class="badge badge-warning animate-pulse">Loading...</span>
                  <span v-else :class="{
                    'badge badge-success': box.status === 'active',
                    'badge badge-error': box.status === 'inactive' || box.status === 'unknown',
                    'badge badge-warning': !box.status || box.status === 'pending',
                  }">
                    {{ box.status || 'unknown' }}
                  </span>
                </td>
                <td class="px-6 py-4 text-right min-w-[160px]">
                  <template v-if="userInfo && userInfo.team_id && selectedTeamId && userInfo.team_id.toString() === selectedTeamId.toString()">
                    <router-link :to="`/console/${encodeURIComponent((teams.find(t => t.name.toString() === selectedTeamId.toString())?.name || ''))}/${encodeURIComponent(box.name)}`" class="btn-secondary w-full block text-center">
                      Console
                    </router-link>
                  </template>
                  <template v-else>
                    <button class="btn-secondary w-full block text-center opacity-50 cursor-not-allowed" disabled>
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
