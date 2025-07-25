<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { apiService } from '@/services/api'
import { ExclamationTriangleIcon } from '@heroicons/vue/24/outline'
import type { LoginUserQuery, IdentitySourcesResponse } from '@/types'
import { IdentitySources } from '@/types'
import Fire from '@/components/Fire.vue'

const route = useRoute()
const loading = ref(false)
const error = ref('')
const message = ref('')
const providerName = import.meta.env.VITE_OIDC_PROVIDER_NAME || ref('Sign-In with OIDC')
const passwordLoading = ref(false)
const identitySources = ref<IdentitySources[]>([])

// Form data for password login
const loginForm = ref<LoginUserQuery>({
  username: '',
  password: ''
})

const errorMessages: Record<string, string> = {
  pkce: 'Authentication failed: PKCE verification error',
  csrf: 'Authentication failed: CSRF token mismatch',
  team: 'Authentication failed: No valid teamd found for your account',
  register: 'Authentication failed: Failed to register user',
  userinfo: 'Authentication failed: Failed to retrieve user information',
  token: 'Authentication failed: Failed to exchange authorization code',
  internal_error: 'An internal error occurred. Please try again.',
  invalid_credentials: 'Invalid username or password. Please try again.'
}

const messages: Record<string, string> = {
  team_switched: 'You have successfully switched teams. Please log in again.',
  registered: 'You have successfully registered. Please log in to continue.',
}

const showPasswordForm = ref(false)

onMounted(async () => {
  // Check for error in query params
  const errorParam = route.query.error as string
  if (errorParam && errorMessages[errorParam]) {
    error.value = errorMessages[errorParam]
  }
  // Check for success message in query params
  const messageParam = route.query.message as string
  if (messageParam && messages[messageParam]) {
    message.value = messages[messageParam]
  }
  try {
    const sourcesResp: IdentitySourcesResponse = await apiService.getIdentitySources()
    identitySources.value = sourcesResp.sources
    // If LocalUserPassword is available, show password form by default
    showPasswordForm.value = identitySources.value.includes(IdentitySources.LocalUserPassword)
  } catch (e) {
    // fallback: show OIDC if error
    identitySources.value = [IdentitySources.OIDC]
    showPasswordForm.value = false
  }
})

const handleLogin = async () => {
  try {
    loading.value = true
    error.value = ''
    const redirectUrl = await apiService.getOAuthRedirectUrl()
    window.location.href = redirectUrl
  } catch (err) {
    console.error('Login failed:', err)
    error.value = 'Failed to initiate login. Please try again.'
    loading.value = false
  }
}

const handlePasswordLogin = async () => {
  try {
    passwordLoading.value = true
    error.value = ''
    // Create URL with query parameters
    const params = new URLSearchParams({
      username: loginForm.value.username,
      password: loginForm.value.password
    })
    // Redirect to the login endpoint with query parameters
    window.location.href = `/api/v1/auth/login?${params.toString()}`
  } catch (err) {
    console.error('Password login failed:', err)
    error.value = 'Failed to login. Please try again.'
    passwordLoading.value = false
  }
}
</script>

<template>
  <div class="min-h-screen flex items-center justify-center bg-gray-50 py-12 px-4 sm:px-6 lg:px-8 relative overflow-hidden">
    <!-- Left Fire -->
    <div class="absolute left-0 top-1/2 transform -translate-y-1/2 z-0" style="width: 300px; height: 400px;">
      <Fire />
    </div>
    <!-- Right Fire -->
    <div class="absolute right-0 top-1/2 transform -translate-y-1/2 z-0" style="width: 300px; height: 400px;">
      <Fire />
    </div>
    <div class="max-w-md w-full space-y-8 relative z-10">
      <div>
        <div class="mx-auto h-12 w-12 flex items-center justify-center rounded-full bg-primary-100">
          <svg class="h-8 w-8 text-black" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.031 9-11.622 0-1.042-.133-2.052-.382-3.016z" />
          </svg>
        </div>
        <h2 class="mt-6 text-center text-3xl font-extrabold text-gray-900">
          Welcome to CARVE
        </h2>
        <p class="mt-2 text-center text-sm text-gray-600">
          Sign in to access the competition platform
        </p>
      </div>

      <div class="mt-8 space-y-6">
        <!-- Error message -->
        <div v-if="error" class="rounded-md bg-red-50 p-4">
          <div class="flex">
            <div class="flex-shrink-0">
              <ExclamationTriangleIcon class="h-5 w-5 text-red-400" />
            </div>
            <div class="ml-3">
              <h3 class="text-sm font-medium text-red-800">
                Authentication Error
              </h3>
              <div class="mt-2 text-sm text-red-700">
                {{ error }}
              </div>
            </div>
          </div>
        </div>
        <!-- Success message -->
        <div v-if="route.query.message" class="rounded-md bg-green-50 p-4">
          <div class="flex">
            <div class="flex-shrink-0">
              <ExclamationTriangleIcon class="h-5 w-5 text-green-400" />
            </div>
            <div class="ml-3">
              <h3 class="text-sm font-medium text-green-800">
                Success
              </h3>
              <div class="mt-2 text-sm text-green-700">
                {{ message }}
              </div>
            </div>
          </div>
        </div>

        <!-- Login form -->
        <div class="card p-6">
          <!-- Show password form if LocalUserPassword is available -->
          <div v-if="showPasswordForm">
            <div class="text-center mb-6">
              <h3 class="text-lg font-medium text-gray-900">Sign in with Password</h3>
              <p class="text-sm text-gray-600 mt-1">
                Enter your username and password
              </p>
            </div>
            <form @submit.prevent="handlePasswordLogin" class="space-y-4">
              <div>
                <label for="username" class="block text-sm font-medium text-gray-700">
                  Username
                </label>
                <input
                  id="username"
                  v-model="loginForm.username"
                  type="text"
                  required
                  class="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm placeholder-gray-400 focus:outline-none focus:ring-primary-500 focus:border-primary-500"
                  placeholder="Enter your username"
                />
              </div>
              <div>
                <label for="password" class="block text-sm font-medium text-gray-700">
                  Password
                </label>
                <input
                  id="password"
                  v-model="loginForm.password"
                  type="password"
                  required
                  class="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm placeholder-gray-400 focus:outline-none focus:ring-primary-500 focus:border-primary-500"
                  placeholder="Enter your password"
                />
              </div>
              <button
                type="submit"
                :disabled="passwordLoading || !loginForm.username || !loginForm.password"
                class="w-full btn-primary flex justify-center items-center py-3 px-4 text-base"
                :class="{ 'opacity-50 cursor-not-allowed': passwordLoading || !loginForm.username || !loginForm.password }"
              >
                <svg v-if="passwordLoading" class="animate-spin -ml-1 mr-3 h-5 w-5 text-white" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                  <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                  <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                </svg>
                {{ passwordLoading ? 'Signing in...' : 'Sign In' }}
              </button>
            </form>
            <!-- If OIDC is also available, show OIDC button below -->
            <div v-if="identitySources.includes(IdentitySources.OIDC)" class="mt-6 text-center">
              <button
                @click="handleLogin"
                :disabled="loading"
                class="w-full btn-primary flex justify-center items-center py-3 px-4 text-base border-2 border-primary-700"
                :class="{ 'opacity-50 cursor-not-allowed': loading }"
              >
                <svg v-if="loading" class="animate-spin -ml-1 mr-3 h-5 w-5 text-white" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                  <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                  <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
                </svg>
                {{ loading ? 'Redirecting...' : providerName }}
              </button>
            </div>
          </div>
          <!-- If only OIDC is available, show OIDC login button -->
          <div v-else class="text-center">
            <p class="text-sm text-gray-600 mb-6">
              Use your organization credentials to sign in
            </p>
            <button
              @click="handleLogin"
              :disabled="loading"
              class="w-full btn-primary flex justify-center items-center py-3 px-4 text-base border-2 border-primary-700"
              :class="{ 'opacity-50 cursor-not-allowed': loading }"
            >
              <svg v-if="loading" class="animate-spin -ml-1 mr-3 h-5 w-5 text-white" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
              </svg>
              {{ loading ? 'Redirecting...' : providerName }}
            </button>
          </div>
        </div>

        <div class="text-center">
          <p class="text-xs text-gray-500">
            By signing in, you agree to participate in the competition according to the rules and guidelines.
          </p>
          <div class="mt-4">
            <router-link
              to="/register"
              class="text-sm text-primary-600 hover:text-primary-500 font-medium underline"
            >
              Don't have an account? Register here
            </router-link>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
