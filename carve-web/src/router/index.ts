import { createRouter, createWebHistory } from 'vue-router'
import { cookieUtils } from '@/utils/cookies'
import Login from '@/views/Login.vue'
import Home from '@/views/Home.vue'
import Scoreboard from '@/views/Scoreboard.vue'
import About from '@/views/About.vue'
import Logout from '@/views/Logout.vue'
import JoinTeam from '@/views/JoinTeam.vue'
import apiService from '@/services/api'
import Compete from '@/views/Compete.vue'

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
    },
    {
      path: '/boxes',
      name: 'Boxes',
      component: () => import('@/views/Boxes.vue'),
      meta: { requiresAuth: true }
    },
    {
      path: '/admin',
      name: 'Admin',
      component: () => import('@/views/Admin.vue'),
      meta: { requiresAuth: true }
    },
    {
      path: '/join_team',
      name: 'JoinTeam',
      component: JoinTeam,
      meta: { requiresAuth: true, hideNavbar: true }
    },
    {
      path: '/console/:team/:box',
      name: 'Console',
      component: () => import('@/views/Console.vue'),
      meta: { requiresAuth: true }
    },
    {
      path: '/compete',
      name: 'Compete',
      component: Compete,
      meta: { requiresAuth: true }
    }
  ],
})

// Navigation guard to check authentication
router.beforeEach(async (to, from, next) => {
  const isLoggedIn = cookieUtils.hasUserInfo()

  if (to.meta.requiresAuth && !isLoggedIn) {
    next('/login')
  } else if (to.path === '/login' && isLoggedIn) {
    next('/')
  } else if (
    isLoggedIn &&
    to.path !== '/join_team' &&
    to.path !== '/logout' &&
    to.path !== '/login' &&
    to.path !== '/admin'
  ) {
    // Check if user is registered for a team
    try {
      const registered = await apiService.isUserRegisteredForAnyTeam()
      console.log('User registration check:', registered)
      if (!registered) {
        return next('/join_team')
      }
    } catch {
      // If check fails, force logout
      return next('/logout')
    }
    next()
  } else {
    next()
  }
})

export default router
