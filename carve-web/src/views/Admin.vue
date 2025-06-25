<script setup lang="ts">
import { ref, onMounted } from 'vue'
import apiService from '@/services/api'
import CompetitionStatus from '@/components/CompetitionStatus.vue'
import type { Team } from '@/types'

const competition = ref<any>(null)
const loading = ref(true)
const error = ref('')
const actionLoading = ref(false)
const teams = ref<Team[]>([])
const selectedTeam = ref<string>('')
const joinCode = ref<string | null>(null)
const joinCodeError = ref('')
const joinCodeLoading = ref(false)

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

onMounted(async () => {
  await fetchCompetition()
  await fetchTeams()
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
      <div v-if="error" class="text-error mt-4">{{ error }}</div>
    </div>
  </div>
</template>
