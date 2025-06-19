<script setup lang="ts">
import { computed } from 'vue'
import { useRoute, RouterView } from 'vue-router'
import Navigation from '@/components/Navigation.vue'
import { cookieUtils } from '@/utils/cookies'

const route = useRoute()
const showNavigation = computed(() => route.path !== '/login' && cookieUtils.hasUserInfo())
</script>

<template>
  <div class="min-h-screen bg-gradient-to-br from-gray-50 to-gray-100">
    <Navigation v-if="showNavigation" />
    <main :class="showNavigation ? 'pt-16' : ''" class="min-h-screen">
      <RouterView />
    </main>
  </div>
</template>

<style scoped>
</style>
