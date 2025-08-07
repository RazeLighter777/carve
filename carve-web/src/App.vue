<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted, watch } from 'vue'
import { useRoute, RouterView } from 'vue-router'
import Navigation from '@/components/Navigation.vue'
import Toast from '@/components/Toast.vue'
import { cookieUtils } from '@/utils/cookies'
import { type ToastNotification } from '@/types'
import { useDarkMode } from '@/composables/useDarkMode'

const route = useRoute()
const showNavigation = computed(() => route.path !== '/login' && cookieUtils.hasUserInfo())
const userInfo = computed(() => cookieUtils.getUserInfo())

// Initialize dark mode
const { initializeTheme } = useDarkMode()

// Toast notification system
const toasts = ref<(ToastNotification & { id: string })[]>([])
let toastSocket: WebSocket | null = null
let nextToastId = 0

const connectToToastSocket = () => {
  const user = userInfo.value
  if (!user || !user.team_name) {
    return
  }

  // Prevent duplicate connections
  if (toastSocket && toastSocket.readyState === WebSocket.OPEN) {
    return
  }

  // Close existing connection if it's in a connecting state
  if (toastSocket && toastSocket.readyState === WebSocket.CONNECTING) {
    toastSocket.close()
  }

  const protocol = window.location.protocol === 'https:' ? 'wss' : 'ws'
  const host = window.location.hostname
  const port = window.location.port ? `:${window.location.port}` : ''
  const params = new URLSearchParams({
    user: user.username,
    team: user.team_name.toString()
  })
  const url = `${protocol}://${host}${port}/api/v1/competition/listen_toasts?${params.toString()}`

  toastSocket = new WebSocket(url)
  
  toastSocket.onopen = () => {
    console.log('Toast WebSocket connected')
  }
  
  toastSocket.onmessage = (event) => {
    try {
      const notification: ToastNotification = JSON.parse(event.data)
      addToast(notification)
    } catch (error) {
      console.error('Failed to parse toast notification:', error)
    }
  }
  
  toastSocket.onclose = () => {
    console.log('Toast WebSocket disconnected')
    // Attempt to reconnect after 5 seconds if user is still logged in
    if (userInfo.value && userInfo.value.team_name) {
      setTimeout(connectToToastSocket, 5000)
    }
  }
  
  toastSocket.onerror = (error) => {
    console.error('Toast WebSocket error:', error)
  }
}

const disconnectToastSocket = () => {
  if (toastSocket) {
    toastSocket.close()
    toastSocket = null
  }
}

const addToast = (notification: ToastNotification) => {
  const toastWithId = {
    ...notification,
    id: `toast-${nextToastId++}`,
  }
  toasts.value.push(toastWithId)
}

const removeToast = (id: string) => {
  const index = toasts.value.findIndex(toast => toast.id === id)
  if (index > -1) {
    toasts.value.splice(index, 1)
  }
}

// Watch for user login/logout changes
watch(userInfo, (newUser, oldUser) => {
  if (newUser && newUser.team_name && (!oldUser || !oldUser.team_name)) {
    // User logged in with a team
    connectToToastSocket()
  } else if ((!newUser || !newUser.team_name) && oldUser && oldUser.team_name) {
    // User logged out or lost team
    disconnectToastSocket()
    toasts.value = [] // Clear existing toasts
  }
}, { immediate: true })

onMounted(() => {
  initializeTheme()
  if (userInfo.value && userInfo.value.team_name) {
    connectToToastSocket()
  }
})

onUnmounted(() => {
  disconnectToastSocket()
})

</script>

<template>
  <div class="min-h-screen bg-gradient-to-br from-gray-50 to-gray-100 dark:from-gray-900 dark:to-gray-800">
    <Navigation v-if="showNavigation" />
    <main :class="showNavigation ? 'pt-16' : ''" class="min-h-screen">
      <RouterView />
    </main>
    
    <!-- Toast notifications container -->
    <div class="fixed top-4 left-1/2 transform -translate-x-1/2 z-50 space-y-2 w-full max-w-2xl px-4">
      <Toast
        v-for="toast in toasts"
        :key="toast.id"
        :notification="toast"
        :auto-close="true"
        @close="removeToast(toast.id)"
      />
    </div>
  </div>
</template>

<style scoped>
</style>
