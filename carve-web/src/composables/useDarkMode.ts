import { ref, watch } from 'vue'

export const isDark = ref(false)

// Track if we've already set up the watcher to avoid duplicates
let watcherInitialized = false

const updateTheme = () => {
  const html = document.documentElement
  if (isDark.value) {
    html.classList.add('dark')
  } else {
    html.classList.remove('dark')
  }
  
  // Debug logging
  console.log('Dark mode:', isDark.value, 'HTML classes:', html.classList.contains('dark'))
}

// Set up the watcher only once
if (!watcherInitialized && typeof window !== 'undefined') {
  watch(isDark, updateTheme, { immediate: true })
  
  watcherInitialized = true
}

export function useDarkMode() {
  const initializeTheme = () => {
    // Check for saved user preference or default to dark mode
    const savedTheme = localStorage.getItem('theme')
    if (savedTheme) {
      isDark.value = savedTheme === 'dark'
    } else {
      isDark.value = true // Default to dark mode
    }
    updateTheme()
  }

  const toggleDarkMode = () => {
    isDark.value = !isDark.value
    localStorage.setItem('theme', isDark.value ? 'dark' : 'light')
    updateTheme()
  }

  return {
    isDark,
    toggleDarkMode,
    initializeTheme
  }
}
