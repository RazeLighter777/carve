<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { apiService } from '@/services/api'

const code = ref('')
const loading = ref(false)
const error = ref('')
const router = useRouter()

const submit = async () => {
  error.value = ''
  if (!code.value.trim()) {
    error.value = 'Please enter a team code.'
    return
  }
  loading.value = true
  try {
    await apiService.switchTeam(code.value.trim())
    window.location.href = '/logout' // Redirect to logout after joining
  } catch (e: any) {
    error.value = e?.response?.data?.error || 'Failed to join team. Please check your code.'
  } finally {
    loading.value = false
  }
}

const logout = () => {
  window.location.href = '/logout'
}
</script>

<template>
  <div class="min-h-screen flex items-center justify-center bg-gray-50">
    <div class="w-full max-w-md bg-white rounded-lg shadow-lg p-8">
      <h1 class="text-2xl font-bold mb-4 text-center">Join a Team</h1>
      <p class="mb-6 text-gray-600 text-center">Enter your team code to join a team and participate in the competition.</p>
      <form @submit.prevent="submit">
        <input
          v-model="code"
          type="text"
          inputmode="numeric"
          pattern="[0-9]*"
          maxlength="10"
          class="w-full px-4 py-2 border rounded mb-4 focus:outline-none focus:ring-2 focus:ring-primary"
          placeholder="Team Code"
          :disabled="loading"
        />
        <button
          type="submit"
          class="w-full btn-primary py-2 font-semibold rounded"
          :disabled="loading"
        >
          {{ loading ? 'Joining...' : 'Join Team' }}
        </button>
        <div v-if="error" class="text-red-600 mt-4 text-center">{{ error }}</div>
      </form>
      <button @click="logout" class="w-full btn-secondary mt-6">Logout</button>
    </div>
  </div>
</template>
