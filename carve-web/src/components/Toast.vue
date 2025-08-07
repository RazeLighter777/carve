<script setup lang="ts">
import { computed, defineProps, defineEmits, onMounted, onUnmounted } from 'vue'
import { ToastSeverity, type ToastNotification } from '@/types'

const props = defineProps<{
  notification: ToastNotification
  autoClose?: boolean
  duration?: number
}>()

const emit = defineEmits<{
  close: []
}>()

let timeoutId: number | null = null

const severityClass = computed(() => {
  switch (props.notification.severity) {
    case ToastSeverity.Info:
      return 'border-blue-400 bg-blue-50 dark:bg-blue-900/30 text-blue-800 dark:text-blue-200'
    case ToastSeverity.Warning:
      return 'border-yellow-400 bg-yellow-50 dark:bg-yellow-900/30 text-yellow-800 dark:text-yellow-200'
    case ToastSeverity.Error:
      return 'border-red-400 bg-red-50 dark:bg-red-900/30 text-red-800 dark:text-red-200'
    default:
      return 'border-gray-400 bg-gray-50 dark:bg-gray-800 text-gray-800 dark:text-gray-200'
  }
})

const iconClass = computed(() => {
  switch (props.notification.severity) {
    case ToastSeverity.Info:
      return 'text-blue-400'
    case ToastSeverity.Warning:
      return 'text-yellow-400'
    case ToastSeverity.Error:
      return 'text-red-400'
    default:
      return 'text-gray-400'
  }
})

const iconPath = computed(() => {
  switch (props.notification.severity) {
    case ToastSeverity.Info:
      return 'M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z'
    case ToastSeverity.Warning:
      return 'M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.082 16.5c-.77.833.192 2.5 1.732 2.5z'
    case ToastSeverity.Error:
      return 'M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z'
    default:
      return 'M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z'
  }
})

const handleClose = () => {
  emit('close')
}

// Auto-close functionality
onMounted(() => {
  if (props.autoClose !== false) {
    const timeout = props.duration || 5000
    timeoutId = window.setTimeout(() => {
      emit('close')
    }, timeout)
  }
})

onUnmounted(() => {
  if (timeoutId) {
    clearTimeout(timeoutId)
  }
})
</script>

<template>
  <div 
    class="w-full bg-white dark:bg-gray-800 shadow-lg rounded-lg pointer-events-auto border-l-4 transform transition-all duration-300 ease-in-out"
    :class="severityClass"
  >
    <div class="p-4">
      <div class="flex items-start">
        <div class="flex-shrink-0">
          <svg 
            class="h-6 w-6" 
            :class="iconClass"
            fill="none" 
            viewBox="0 0 24 24" 
            stroke="currentColor"
          >
            <path 
              stroke-linecap="round" 
              stroke-linejoin="round" 
              stroke-width="2" 
              :d="iconPath"
            />
          </svg>
        </div>
        <div class="ml-3 w-0 flex-1">
          <p class="text-sm font-medium">{{ notification.title }}</p>
          <p class="mt-1 text-sm opacity-90">{{ notification.message }}</p>
          <div v-if="notification.user || notification.team" class="mt-2 text-xs opacity-75">
            <span v-if="notification.user">User: {{ notification.user }}</span>
            <span v-if="notification.team" :class="{ 'ml-2': notification.user }">
              Team: {{ notification.team }}
            </span>
          </div>
        </div>
        <div class="ml-4 flex-shrink-0 flex">
          <button
            @click="handleClose"
            class="inline-flex text-gray-400 dark:text-gray-500 hover:text-gray-600 dark:hover:text-gray-300 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500"
          >
            <span class="sr-only">Close</span>
            <svg class="h-5 w-5" viewBox="0 0 20 20" fill="currentColor">
              <path 
                fill-rule="evenodd" 
                d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z" 
                clip-rule="evenodd" 
              />
            </svg>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
