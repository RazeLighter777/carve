<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ExclamationTriangleIcon } from '@heroicons/vue/24/outline'
import type { RegistrationQuery } from '@/types'

const route = useRoute()
const router = useRouter()
const loading = ref(false)
const error = ref('')
const success = ref('')

// Form data for registration
const registrationForm = ref<RegistrationQuery>({
  username: '',
  password: '',
  email: '',
  team_join_code: undefined
})

const errorMessages: Record<string, string> = {
  internal_error: 'An internal error occurred. Please try again.',
  username_exists: 'This username is already taken. Please choose a different one.',
  password_requirements_not_met: 'Password does not meet requirements. Please use a stronger password.'
}

const successMessages: Record<string, string> = {
  registered: 'Registration successful! You can now sign in with your credentials.'
}

onMounted(async () => {
  // Check for error in query params
  const errorParam = route.query.error as string
  if (errorParam && errorMessages[errorParam]) {
    error.value = errorMessages[errorParam]
  }
  
  // Check for success in query params
  const successParam = route.query.success as string
  if (successParam && successMessages[successParam]) {
    success.value = successMessages[successParam]
  }
})

const handleRegistration = async () => {
  try {
    loading.value = true
    error.value = ''
    success.value = ''
    
    // Create URL with query parameters
    const params = new URLSearchParams({
      username: registrationForm.value.username,
      password: registrationForm.value.password,
      email: registrationForm.value.email
    })
    
    // Add team join code if provided
    if (registrationForm.value.team_join_code) {
      params.append('team_join_code', registrationForm.value.team_join_code.toString())
    }
    
    // Redirect to the registration endpoint with query parameters
    window.location.href = `/api/v1/auth/register?${params.toString()}`
  } catch (err) {
    console.error('Registration failed:', err)
    error.value = 'Failed to register. Please try again.'
    loading.value = false
  }
}

const goToLogin = () => {
  router.push('/login')
}
</script>

<template>
  <div class="min-h-screen flex items-center justify-center bg-gray-50 py-12 px-4 sm:px-6 lg:px-8">
    <div class="max-w-md w-full space-y-8">
      <div>
        <div class="mx-auto h-12 w-12 flex items-center justify-center rounded-full bg-primary-100">
          <svg class="h-8 w-8 text-black" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z" />
          </svg>
        </div>
        <h2 class="mt-6 text-center text-3xl font-extrabold text-gray-900">
          Create Account
        </h2>
        <p class="mt-2 text-center text-sm text-gray-600">
          Join the CARVE competition platform
        </p>
      </div>

      <div class="mt-8 space-y-6">
        <!-- Success message -->
        <div v-if="success" class="rounded-md bg-green-50 p-4">
          <div class="flex">
            <div class="flex-shrink-0">
              <svg class="h-5 w-5 text-green-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
            </div>
            <div class="ml-3">
              <h3 class="text-sm font-medium text-green-800">
                Registration Successful
              </h3>
              <div class="mt-2 text-sm text-green-700">
                {{ success }}
              </div>
              <div class="mt-4">
                <button
                  @click="goToLogin"
                  class="text-sm text-green-800 font-medium hover:text-green-600 underline"
                >
                  Go to Login
                </button>
              </div>
            </div>
          </div>
        </div>

        <!-- Error message -->
        <div v-if="error" class="rounded-md bg-red-50 p-4">
          <div class="flex">
            <div class="flex-shrink-0">
              <ExclamationTriangleIcon class="h-5 w-5 text-red-400" />
            </div>
            <div class="ml-3">
              <h3 class="text-sm font-medium text-red-800">
                Registration Error
              </h3>
              <div class="mt-2 text-sm text-red-700">
                {{ error }}
              </div>
            </div>
          </div>
        </div>

        <!-- Registration form -->
        <div v-if="!success" class="card p-6">
          <div class="text-center mb-6">
            <h3 class="text-lg font-medium text-gray-900">Register for CARVE</h3>
            <p class="text-sm text-gray-600 mt-1">
              Fill in your details to create an account
            </p>
          </div>
          
          <form @submit.prevent="handleRegistration" class="space-y-4">
            <div>
              <label for="username" class="block text-sm font-medium text-gray-700">
                Username *
              </label>
              <input
                id="username"
                v-model="registrationForm.username"
                type="text"
                required
                class="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm placeholder-gray-400 focus:outline-none focus:ring-primary-500 focus:border-primary-500"
                placeholder="Choose a username"
              />
            </div>
            
            <div>
              <label for="email" class="block text-sm font-medium text-gray-700">
                Email Address *
              </label>
              <input
                id="email"
                v-model="registrationForm.email"
                type="email"
                required
                class="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm placeholder-gray-400 focus:outline-none focus:ring-primary-500 focus:border-primary-500"
                placeholder="Enter your email address"
              />
            </div>
            
            <div>
              <label for="password" class="block text-sm font-medium text-gray-700">
                Password *
              </label>
              <input
                id="password"
                v-model="registrationForm.password"
                type="password"
                required
                class="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm placeholder-gray-400 focus:outline-none focus:ring-primary-500 focus:border-primary-500"
                placeholder="Create a strong password"
              />
              <p class="mt-1 text-xs text-gray-500">
                Use a strong password with a mix of letters, numbers, and special characters
              </p>
            </div>
            
            <div>
              <label for="team_join_code" class="block text-sm font-medium text-gray-700">
                Team Join Code (Optional)
              </label>
              <input
                id="team_join_code"
                v-model.number="registrationForm.team_join_code"
                type="number"
                class="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm placeholder-gray-400 focus:outline-none focus:ring-primary-500 focus:border-primary-500"
                placeholder="Enter team join code (if you have one)"
              />
              <p class="mt-1 text-xs text-gray-500">
                Leave blank if you don't have a team join code yet
              </p>
            </div>
            
            <button
              type="submit"
              :disabled="loading || !registrationForm.username || !registrationForm.email || !registrationForm.password"
              class="w-full btn-primary flex justify-center items-center py-3 px-4 text-base"
              :class="{ 'opacity-50 cursor-not-allowed': loading || !registrationForm.username || !registrationForm.email || !registrationForm.password }"
            >
              <svg v-if="loading" class="animate-spin -ml-1 mr-3 h-5 w-5 text-white" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
              </svg>
              {{ loading ? 'Creating Account...' : 'Create Account' }}
            </button>
          </form>
          
          <div class="mt-6 text-center">
            <p class="text-sm text-gray-600">
              Already have an account?
              <button
                @click="goToLogin"
                class="font-medium text-primary-600 hover:text-primary-500 underline"
              >
                Sign in here
              </button>
            </p>
          </div>
        </div>

        <div class="text-center">
          <p class="text-xs text-gray-500">
            By creating an account, you agree to participate in the competition according to the rules and guidelines.
          </p>
        </div>
      </div>
    </div>
  </div>
</template>
