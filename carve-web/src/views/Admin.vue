<script setup lang="ts">
import { ref, onMounted } from 'vue'
import apiService from '@/services/api'
import CompetitionStatus from '@/components/CompetitionStatus.vue'

const competition = ref<any>(null)
const loading = ref(true)
const error = ref('')
const actionLoading = ref(false)

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

onMounted(fetchCompetition)
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
      <div v-if="error" class="text-error mt-4">{{ error }}</div>
    </div>
  </div>
</template>
