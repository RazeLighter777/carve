import { createRouter, createWebHistory } from 'vue-router'
import HomeView from '../views/HomeView.vue'

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      // displays the home page of the competition, containing the competition description, the time remaining, and the current points total.
      // if the user is not logged in, they will be redirected to the login page.
      path: '/',
      name: 'home',
      component: HomeView,
    },
    {
      // links to the about page, which contains information about the CARVE project.
      path: '/about',
      name: 'about',
      component: () => import('../views/AboutView.vue'),
    },
    {
      // links to oauth2 login page
      path: '/login',
      name: 'login',
      component: () => import('../views/LoginView.vue'),
    },
    {
      // links to the leaderboard of all the teams competing.
      path: '/leaderboard',
      name: 'leaderboard',
      component: () => import('../views/LeaderboardView.vue'),
    },
    {
      path: '/settings',
      name: 'settings',
      component: () => import('../views/SettingsView.vue'),
    }
  ],
})

export default router
