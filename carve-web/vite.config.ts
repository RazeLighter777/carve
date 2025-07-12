import { fileURLToPath, URL } from 'node:url'

import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import vueDevTools from 'vite-plugin-vue-devtools'
import tailwindcss from '@tailwindcss/vite'

// https://vite.dev/config/
export default defineConfig({
  server: {
    port : 4173,
    proxy : {
      '/api/v1': {
        target: 'http://localhost:5000',
      },
      '/novnc': {
        target: 'http://localhost:6080',
        ws: true,
      },
      '/xtermjs': {
        target: 'http://localhost:6080',
        ws: true,
      }
    },
    allowedHosts: [
      'localhost',
      'carve.prizrak.me'
    ]
  },
  plugins: [
    vue(),
    vueDevTools(),
    tailwindcss()
  ],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url))
    },
  },
})
