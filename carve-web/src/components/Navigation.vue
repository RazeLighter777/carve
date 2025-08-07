<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { RouterLink } from 'vue-router'
import { cookieUtils } from '@/utils/cookies'
import { useDarkMode } from '@/composables/useDarkMode'
import { 
  Bars3Icon, 
  XMarkIcon, 
  HomeIcon, 
  TrophyIcon, 
  ChartBarIcon, 
  InformationCircleIcon,
  ArrowRightOnRectangleIcon,
  CubeTransparentIcon,
  FlagIcon,
  ChatBubbleLeftRightIcon,
  SunIcon,
  MoonIcon
} from '@heroicons/vue/24/outline'

const userInfo = ref<any>(null)
const isMenuOpen = ref(false)
const isAdmin = ref(false)
const { isDark, toggleDarkMode } = useDarkMode()

onMounted(async () => {
  userInfo.value = cookieUtils.getUserInfo()
  try {
    isAdmin.value = userInfo.value?.is_admin || false
    console.log('User is admin:', isAdmin.value)
  } catch (e) {
    isAdmin.value = false
  }
})

const toggleMenu = () => {
  isMenuOpen.value = !isMenuOpen.value
}

const closeMenu = () => {
  isMenuOpen.value = false
}
</script>

<template>  <nav class="bg-white/80 dark:bg-gray-800/80 backdrop-blur-sm shadow-xl border-b border-gray-200/60 dark:border-gray-700/60 fixed w-full top-0 z-50">
    <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
      <div class="flex justify-between h-16">
        <div class="flex items-center">
          <div class="flex-shrink-0">
            <div class="flex items-center">
              <div class="w-8 h-8 bg-gradient-to-br from-primary-500 to-primary-700 rounded-lg flex items-center justify-center mr-3">
                <span class="text-white font-bold text-xl">🎃</span>
              </div>
              <h1 class="text-xl font-bold bg-gradient-to-r from-primary-600 to-primary-800 text-gray-700 dark:text-white bg-clip-text">CARVE</h1>
            </div>
          </div>
          
          <!-- Desktop Navigation -->
          <div class="hidden md:ml-6 md:flex md:space-x-8">
            <RouterLink 
              to="/" 
              class="flex items-center px-3 py-2 rounded-md text-sm font-medium text-gray-700 dark:text-gray-300 hover:text-black dark:hover:text-white hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
              active-class="text-black dark:text-white bg-white dark:bg-gray-700"
            >
              <HomeIcon class="w-4 h-4 mr-2" />
              Home
            </RouterLink>
            <RouterLink 
              to="/compete" 
              class="flex items-center px-3 py-2 rounded-md text-sm font-medium text-gray-700 dark:text-gray-300 hover:text-black dark:hover:text-white hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
              active-class="text-black dark:text-white bg-white dark:bg-gray-700"
            >
              <FlagIcon class="w-4 h-4 mr-2" />
              Flags/Checks
            </RouterLink>
            <RouterLink 
              to="/scoreboard" 
              class="flex items-center px-3 py-2 rounded-md text-sm font-medium text-gray-700 dark:text-gray-300 hover:text-black dark:hover:text-white hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
              active-class="text-black dark:text-white bg-white dark:bg-gray-700"
            >
              <ChartBarIcon class="w-4 h-4 mr-2" />
              Scoreboard
            </RouterLink>
            <RouterLink 
              to="/about" 
              class="flex items-center px-3 py-2 rounded-md text-sm font-medium text-gray-700 dark:text-gray-300 hover:text-black dark:hover:text-white hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
              active-class="text-black dark:text-white bg-white dark:bg-gray-700"
            >
              <InformationCircleIcon class="w-4 h-4 mr-2" />
              About
            </RouterLink>
            <RouterLink 
              to="/boxes" 
              class="flex items-center px-3 py-2 rounded-md text-sm font-medium text-gray-700 dark:text-gray-300 hover:text-black dark:hover:text-white hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
              active-class="text-black dark:text-white bg-white dark:bg-gray-700"
            >
              <CubeTransparentIcon class="w-4 h-4 mr-2" />
              Boxes
            </RouterLink>
            <RouterLink 
              to="/tickets" 
              class="flex items-center px-3 py-2 rounded-md text-sm font-medium text-gray-700 dark:text-gray-300 hover:text-black dark:hover:text-white hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
              active-class="text-black dark:text-white bg-white dark:bg-gray-700"
            >
              <ChatBubbleLeftRightIcon class="w-4 h-4 mr-2" />
              Tickets
            </RouterLink>
            <RouterLink 
              v-if="isAdmin" 
              to="/admin" 
              class="flex items-center px-3 py-2 rounded-md text-sm font-medium text-gray-700 dark:text-gray-300 hover:text-black dark:hover:text-white hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
              active-class="text-black dark:text-white bg-white dark:bg-gray-700"
            >
              <svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4 mr-2" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" /></svg>
              Admin
            </RouterLink>
          </div>
        </div>

        <!-- User info and logout -->
        <div class="hidden md:flex md:items-center md:space-x-4">
          <label class="inline-flex items-center cursor-pointer">
            <input type="checkbox" value="" class="sr-only peer" @change="toggleDarkMode" :checked="isDark">
            <div class="relative w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 dark:peer-focus:ring-blue-800 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full rtl:peer-checked:after:-translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:start-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-blue-600 dark:peer-checked:bg-blue-600"></div>
            <span class="ms-3 text-sm font-medium text-gray-900 dark:text-gray-300">Dark/Light</span>
          </label>
          
          <div class="text-sm text-gray-700 dark:text-gray-300" v-if="userInfo">
            Welcome, <span class="font-medium">{{ userInfo.username }}</span>
            <span v-if="userInfo.team_name" class="text-gray-500 dark:text-gray-400 ml-1">({{ userInfo.team_name }})</span>
          </div>
          <RouterLink 
            to="/logout" 
            class="flex items-center px-3 py-2 rounded-md text-sm font-medium text-gray-700 dark:text-gray-300 hover:text-red-600 dark:hover:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/30 transition-colors"
          >
            <ArrowRightOnRectangleIcon class="w-4 h-4 mr-2" />
            Logout
          </RouterLink>
        </div>

        <!-- Mobile menu button -->
        <div class="md:hidden flex items-center space-x-2">
          <!-- Dark mode toggle for mobile -->
          <button
            @click="toggleDarkMode"
            class="p-2 rounded-lg text-gray-700 dark:text-gray-300 hover:text-black dark:hover:text-white hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors focus:outline-none focus:ring-2 focus:ring-primary-500"
            :title="isDark ? 'Switch to light mode' : 'Switch to dark mode'"
          >
            <SunIcon v-if="isDark" class="w-5 h-5" />
            <MoonIcon v-else class="w-5 h-5" />
          </button>
          
          <button
            @click="toggleMenu"
            class="text-gray-700 dark:text-gray-300 hover:text-black dark:hover:text-white focus:outline-none focus:text-black dark:focus:text-white transition-colors"
          >
            <Bars3Icon v-if="!isMenuOpen" class="w-6 h-6" />
            <XMarkIcon v-else class="w-6 h-6" />
          </button>
        </div>
      </div>
    </div>

    <!-- Mobile Navigation Menu -->
    <div v-if="isMenuOpen" class="md:hidden">
      <div class="px-2 pt-2 pb-3 space-y-1 sm:px-3 bg-white dark:bg-gray-800 border-t dark:border-gray-700">
        <RouterLink 
          to="/" 
          @click="closeMenu"
          class="flex items-center px-3 py-2 rounded-md text-base font-medium text-gray-700 dark:text-gray-300 hover:text-black dark:hover:text-white hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
          active-class="text-black dark:text-white bg-white dark:bg-gray-700"
        >
          <HomeIcon class="w-5 h-5 mr-3" />
          Home
        </RouterLink>
        <RouterLink 
          to="/compete" 
          @click="closeMenu"
          class="flex items-center px-3 py-2 rounded-md text-base font-medium text-gray-700 dark:text-gray-300 hover:text-black dark:hover:text-white hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
          active-class="text-black dark:text-white bg-white dark:bg-gray-700"
        >
          <FlagIcon class="w-5 h-5 mr-3" />
          Flags/Checks
        </RouterLink>
        <RouterLink 
          to="/scoreboard" 
          @click="closeMenu"
          class="flex items-center px-3 py-2 rounded-md text-base font-medium text-gray-700 dark:text-gray-300 hover:text-black dark:hover:text-white hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
          active-class="text-black dark:text-white bg-white dark:bg-gray-700"
        >
          <ChartBarIcon class="w-5 h-5 mr-3" />
          Scoreboard
        </RouterLink>
        <RouterLink 
          to="/boxes" 
          @click="closeMenu"
          class="flex items-center px-3 py-2 rounded-md text-base font-medium text-gray-700 dark:text-gray-300 hover:text-black dark:hover:text-white hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
          active-class="text-black dark:text-white bg-white dark:bg-gray-700"
        >
          <CubeTransparentIcon class="w-5 h-5 mr-3" />
          Boxes
        </RouterLink>
        <RouterLink 
          to="/tickets" 
          @click="closeMenu"
          class="flex items-center px-3 py-2 rounded-md text-base font-medium text-gray-700 dark:text-gray-300 hover:text-black dark:hover:text-white hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
          active-class="text-black dark:text-white bg-white dark:bg-gray-700"
        >
          <ChatBubbleLeftRightIcon class="w-5 h-5 mr-3" />
          Tickets
        </RouterLink>
        <RouterLink 
          to="/about" 
          @click="closeMenu"
          class="flex items-center px-3 py-2 rounded-md text-base font-medium text-gray-700 dark:text-gray-300 hover:text-black dark:hover:text-white hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
          active-class="text-black dark:text-white bg-white dark:bg-gray-700"
        >
          <InformationCircleIcon class="w-5 h-5 mr-3" />
          About
        </RouterLink>
        <RouterLink 
          v-if="isAdmin" 
          to="/admin"
          @click="closeMenu"
          class="flex items-center px-3 py-2 rounded-md text-base font-medium text-gray-700 dark:text-gray-300 hover:text-black dark:hover:text-white hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
          active-class="text-black dark:text-white bg-white dark:bg-gray-700"
        >
          <svg xmlns="http://www.w3.org/2000/svg" class="w-5 h-5 mr-3" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" /></svg>
          Admin
        </RouterLink>

        <div class="border-t dark:border-gray-700 pt-4" v-if="userInfo">
          <div class="px-3 py-2 text-sm text-gray-700 dark:text-gray-300">
            Welcome, <span class="font-medium">{{ userInfo.username }}</span>
            <div v-if="userInfo.team_name" class="text-gray-500 dark:text-gray-400">Team: {{ userInfo.team_name }}</div>
          </div>
          <RouterLink 
            to="/logout" 
            @click="closeMenu"
            class="flex items-center px-3 py-2 rounded-md text-base font-medium text-gray-700 dark:text-gray-300 hover:text-red-600 dark:hover:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/30 transition-colors"
          >
            <ArrowRightOnRectangleIcon class="w-5 h-5 mr-3" />
            Logout
          </RouterLink>
        </div>
      </div>
    </div>
  </nav>
</template>
