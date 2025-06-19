<script setup lang="ts">
import { onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { cookieUtils } from '@/utils/cookies'

const router = useRouter()

onMounted(() => {
  // Clear auth cookies
  cookieUtils.clearAuth()

  // call /api/v1/logout endpoint
  fetch('/api/v1/logout', {
    method: 'GET',
    credentials: 'include' // Ensure cookies are sent
  })
  
  // Redirect to login
  router.push('/login')
})
</script>

<template>
  <div class="min-h-screen flex items-center justify-center bg-gray-50">
    <div class="text-center">
      <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-gray-300 mx-auto mb-4"></div>
      <p class="text-gray-600">Logging out...</p>
    </div>
  </div>
</template>
