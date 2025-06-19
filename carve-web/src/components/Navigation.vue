<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { RouterLink } from 'vue-router'
import { cookieUtils } from '@/utils/cookies'
import { 
  Bars3Icon, 
  XMarkIcon, 
  HomeIcon, 
  TrophyIcon, 
  ChartBarIcon, 
  InformationCircleIcon,
  ArrowRightOnRectangleIcon
} from '@heroicons/vue/24/outline'

const userInfo = ref<any>(null)
const isMenuOpen = ref(false)

onMounted(() => {
  userInfo.value = cookieUtils.getUserInfo()
})

const toggleMenu = () => {
  isMenuOpen.value = !isMenuOpen.value
}

const closeMenu = () => {
  isMenuOpen.value = false
}
</script>

<template>  <nav class="bg-white/80 backdrop-blur-sm shadow-xl border-b border-gray-200/60 fixed w-full top-0 z-50">
    <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
      <div class="flex justify-between h-16">
        <div class="flex items-center">
          <div class="flex-shrink-0">
            <div class="flex items-center">
              <div class="w-8 h-8 bg-gradient-to-br from-primary-500 to-primary-700 rounded-lg flex items-center justify-center mr-3">
                <span class="text-white font-bold text-sm">C</span>
              </div>
              <h1 class="text-xl font-bold bg-gradient-to-r from-primary-600 to-primary-800 bg-clip-text text-transparent">CARVE</h1>
            </div>
          </div>
          
          <!-- Desktop Navigation -->
          <div class="hidden md:ml-6 md:flex md:space-x-8">
            <RouterLink 
              to="/" 
              class="flex items-center px-3 py-2 rounded-md text-sm font-medium text-gray-700 hover:text-black hover:bg-gray-50 transition-colors"
              active-class="text-black bg-white"
            >
              <HomeIcon class="w-4 h-4 mr-2" />
              Home
            </RouterLink>
            <RouterLink 
              to="/leaderboard" 
              class="flex items-center px-3 py-2 rounded-md text-sm font-medium text-gray-700 hover:text-black hover:bg-gray-50 transition-colors"
              active-class="text-black bg-white"
            >
              <TrophyIcon class="w-4 h-4 mr-2" />
              Leaderboard
            </RouterLink>
            <RouterLink 
              to="/scoreboard" 
              class="flex items-center px-3 py-2 rounded-md text-sm font-medium text-gray-700 hover:text-black hover:bg-gray-50 transition-colors"
              active-class="text-black bg-white"
            >
              <ChartBarIcon class="w-4 h-4 mr-2" />
              Scoreboard
            </RouterLink>
            <RouterLink 
              to="/about" 
              class="flex items-center px-3 py-2 rounded-md text-sm font-medium text-gray-700 hover:text-black hover:bg-gray-50 transition-colors"
              active-class="text-black bg-white"
            >
              <InformationCircleIcon class="w-4 h-4 mr-2" />
              About
            </RouterLink>
          </div>
        </div>

        <!-- User info and logout -->
        <div class="hidden md:flex md:items-center md:space-x-4">
          <div class="text-sm text-gray-700" v-if="userInfo">
            Welcome, <span class="font-medium">{{ userInfo.username }}</span>
            <span v-if="userInfo.team_name" class="text-gray-500 ml-1">({{ userInfo.team_name }})</span>
          </div>
          <RouterLink 
            to="/logout" 
            class="flex items-center px-3 py-2 rounded-md text-sm font-medium text-gray-700 hover:text-red-600 hover:bg-red-50 transition-colors"
          >
            <ArrowRightOnRectangleIcon class="w-4 h-4 mr-2" />
            Logout
          </RouterLink>
        </div>

        <!-- Mobile menu button -->
        <div class="md:hidden flex items-center">
          <button
            @click="toggleMenu"
            class="text-gray-700 hover:text-black focus:outline-none focus:text-black transition-colors"
          >
            <Bars3Icon v-if="!isMenuOpen" class="w-6 h-6" />
            <XMarkIcon v-else class="w-6 h-6" />
          </button>
        </div>
      </div>
    </div>

    <!-- Mobile Navigation Menu -->
    <div v-if="isMenuOpen" class="md:hidden">
      <div class="px-2 pt-2 pb-3 space-y-1 sm:px-3 bg-white border-t">
        <RouterLink 
          to="/" 
          @click="closeMenu"
          class="flex items-center px-3 py-2 rounded-md text-base font-medium text-gray-700 hover:text-black hover:bg-gray-50 transition-colors"
          active-class="text-black bg-white"
        >
          <HomeIcon class="w-5 h-5 mr-3" />
          Home
        </RouterLink>
        <RouterLink 
          to="/leaderboard" 
          @click="closeMenu"
          class="flex items-center px-3 py-2 rounded-md text-base font-medium text-gray-700 hover:text-black hover:bg-gray-50 transition-colors"
          active-class="text-black bg-white"
        >
          <TrophyIcon class="w-5 h-5 mr-3" />
          Leaderboard
        </RouterLink>
        <RouterLink 
          to="/scoreboard" 
          @click="closeMenu"
          class="flex items-center px-3 py-2 rounded-md text-base font-medium text-gray-700 hover:text-black hover:bg-gray-50 transition-colors"
          active-class="text-black bg-white"
        >
          <ChartBarIcon class="w-5 h-5 mr-3" />
          Scoreboard
        </RouterLink>
        <RouterLink 
          to="/about" 
          @click="closeMenu"
          class="flex items-center px-3 py-2 rounded-md text-base font-medium text-gray-700 hover:text-black hover:bg-gray-50 transition-colors"
          active-class="text-black bg-white"
        >
          <InformationCircleIcon class="w-5 h-5 mr-3" />
          About
        </RouterLink>
        
        <div class="border-t pt-4" v-if="userInfo">
          <div class="px-3 py-2 text-sm text-gray-700">
            Welcome, <span class="font-medium">{{ userInfo.username }}</span>
            <div v-if="userInfo.team_name" class="text-gray-500">Team: {{ userInfo.team_name }}</div>
          </div>
          <RouterLink 
            to="/logout" 
            @click="closeMenu"
            class="flex items-center px-3 py-2 rounded-md text-base font-medium text-gray-700 hover:text-red-600 hover:bg-red-50 transition-colors"
          >
            <ArrowRightOnRectangleIcon class="w-5 h-5 mr-3" />
            Logout
          </RouterLink>
        </div>
      </div>
    </div>
  </nav>
</template>
