import { ref, watch } from 'vue'

export const isDark = ref(false)

export function useDarkMode() {
  const initializeTheme = () => {
    // Check for saved user preference or default to system preference
    const savedTheme = localStorage.getItem('theme')
    if (savedTheme) {
      isDark.value = savedTheme === 'dark'
    } else {
      isDark.value = window.matchMedia('(prefers-color-scheme: dark)').matches
    }
    updateTheme()
  }

  const toggleDarkMode = () => {
    isDark.value = !isDark.value
    localStorage.setItem('theme', isDark.value ? 'dark' : 'light')
    updateTheme()
  }

  const updateTheme = () => {
    if (isDark.value) {
      document.documentElement.classList.add('dark')
    } else {
      document.documentElement.classList.remove('dark')
    }
  }

  // Watch for system theme changes
  const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')
  const handleSystemThemeChange = (e: MediaQueryListEvent) => {
    if (!localStorage.getItem('theme')) {
      isDark.value = e.matches
      updateTheme()
    }
  }

  mediaQuery.addEventListener('change', handleSystemThemeChange)

  return {
    isDark,
    toggleDarkMode,
    initializeTheme
  }
}
