import { createRouter, createWebHistory } from 'vue-router'
import { cookieUtils } from '@/utils/cookies'
import Login from '@/views/Login.vue'
import Home from '@/views/Home.vue'
import Leaderboard from '@/views/Leaderboard.vue'
import Scoreboard from '@/views/Scoreboard.vue'
import About from '@/views/About.vue'
import Logout from '@/views/Logout.vue'

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: '/login',
      name: 'Login',
      component: Login,
      meta: { requiresAuth: false }
    },
    {
      path: '/logout',
      name: 'Logout',
      component: Logout,
      meta: { requiresAuth: false }
    },
    {
      path: '/',
      name: 'Home',
      component: Home,
      meta: { requiresAuth: true }
    },
    {
      path: '/leaderboard',
      name: 'Leaderboard',
      component: Leaderboard,
      meta: { requiresAuth: true }
    },
    {
      path: '/scoreboard',
      name: 'Scoreboard',
      component: Scoreboard,
      meta: { requiresAuth: true }
    },
    {
      path: '/about',
      name: 'About',
      component: About,
      meta: { requiresAuth: true }
    }
  ],
})

// Navigation guard to check authentication
router.beforeEach((to, from, next) => {
  const isLoggedIn = cookieUtils.hasUserInfo()
  
  if (to.meta.requiresAuth && !isLoggedIn) {
    next('/login')
  } else if (to.path === '/login' && isLoggedIn) {
    next('/')
  } else {
    next()
  }
})

export default router
