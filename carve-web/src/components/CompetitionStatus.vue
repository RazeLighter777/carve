<script setup lang="ts">
import { computed, defineProps, ref, onMounted, onUnmounted } from 'vue'
import { CompetitionStatus, type CompetitionState } from '@/types'

const props = defineProps({
  competition: {
    type: Object,
    required: true
  }
})

const statusText = computed(() => {
  return props.competition.status.charAt(0).toUpperCase() + props.competition.status.slice(1)
})

const statusClass = computed(() => {
    if (props.competition.status == "Active") return 'text-green-600 dark:text-green-400'
    if (props.competition.status == "Unstarted") return 'text-yellow-600 dark:text-yellow-400'
    if (props.competition.status == "Finished") return 'text-red-600 dark:text-red-400'
    console.warn('Unknown competition status:', props.competition.status)
})

const competition = computed(() => props.competition)

// Timer logic
const timeRemaining = ref('')
const timerColor = ref('text-green-600')
let intervalId: number | undefined

function updateTimeRemaining() {
  if (!competition.value.end_time || !competition.value.start_time || competition.value.status !== 'Active') {
    timeRemaining.value = ''
    timerColor.value = ''
    return
  }
  const now = Date.now()
  const end = new Date(competition.value.end_time).getTime()
  let diff = Math.max(0, Math.floor((end - now) / 1000)) // in seconds
  if (diff <= 0) {
    timeRemaining.value = '00:00:00'
    timerColor.value = 'text-red-600'
    // Optionally, emit an event or reload/refresh the page or state
    return
  }
  const hours = Math.floor(diff / 3600)
  const minutes = Math.floor((diff % 3600) / 60)
  const seconds = diff % 60
  timeRemaining.value = `${hours.toString().padStart(2, '0')}:${minutes
    .toString().padStart(2, '0')}:${seconds.toString().padStart(2, '0')}`
  if (diff > 3600) {
    timerColor.value = 'text-green-600 dark:text-green-400'
  } else if (diff > 600) {
    timerColor.value = 'text-yellow-600 dark:text-yellow-400'
  } else {
    timerColor.value = 'text-red-600 dark:text-red-400'
  }
}

onMounted(() => {
  updateTimeRemaining()
  intervalId = window.setInterval(() => {
    updateTimeRemaining()
    // If time is up, clear interval
    if (competition.value.end_time && competition.value.status === 'Active') {
      const now = Date.now()
      const end = new Date(competition.value.end_time).getTime()
      if (now >= end) {
        updateTimeRemaining()
        clearInterval(intervalId)
      }
    }
  }, 1000)
})

onUnmounted(() => {
  if (intervalId) clearInterval(intervalId)
})
</script>

<template>
  <div class="card p-6 mb-6">
    <div class="flex items-center justify-between">
      <div>
        <h2 class="text-xl font-semibold text-heading mb-2">{{competition.name}} Status</h2>
        <div class="text-body">
          <span>Status: </span>
          <span :class="statusClass" class="font-semibold">{{ statusText }}</span>
            <span v-if="competition.start_time" class="ml-4">
                Start Time: {{ new Date(competition.start_time).toLocaleString() }}
            </span>
            <span v-if="competition.end_time" class="ml-4">
                End Time: {{ new Date(competition.end_time).toLocaleString() }}
            </span>
            <span v-if="competition.status === 'Active' && timeRemaining" :class="['ml-4', timerColor, 'font-mono', 'font-bold']">
              Time Remaining: {{ timeRemaining }}
            </span>
        </div>
      </div>
    </div>
  </div>
</template>
