
<script setup lang="ts">
import { computed, defineProps } from 'vue'
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
    console.log(typeof props.competition.status)
    if (props.competition.status == "Active") return 'text-green-600'
    if (props.competition.status == "Unstarted") return 'text-yellow-600'
    if (props.competition.status == "Finished") return 'text-red-600'
    console.warn('Unknown competition status:', props.competition.status)
})

const competition = computed(() => props.competition)

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
                Start Time: {{ new Date(competition.start_time * 1000).toLocaleString() }}
            </span>
            <span v-if="competition.end_time" class="ml-4">
                End Time: {{ new Date(competition.end_time * 1000).toLocaleString() }}
            </span>

        </div>
      </div>
    </div>
  </div>
</template>
