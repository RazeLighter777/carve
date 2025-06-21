<script setup lang="ts">
import { useRoute } from 'vue-router'
import { computed, ref, onMounted, nextTick } from 'vue'
// @ts-ignore
import RFB from '@novnc/novnc/lib/rfb.js'
import { apiService } from '@/services/api'

const route = useRoute()
const boxName = computed(() => route.params.box as string || '')
const teamName = computed(() => route.params.team as string || '')

const statusText = ref('Loading')
const screenEl = ref<HTMLElement | null>(null)
const rfb = ref<any>(null)
const desktopName = ref('')

function status(text: string) {
  statusText.value = text
}

function sendCtrlAltDel() {
  if (rfb.value) {
    rfb.value.sendCtrlAltDel()
  }
}

onMounted(async () => {
  if (!screenEl.value || !boxName.value || !teamName.value) return
  status('Getting console code...')
  let code = ''
  try {
    code = await apiService.getTeamConsoleCode()
  } catch (e) {
    status('Failed to get console code')
    return
  }

  // Wait for DOM to be fully rendered
  await nextTick()

  // Build the websocket URL correctly
  const protocol = window.location.protocol === 'https:' ? 'wss' : 'ws'
  const host = window.location.hostname
  const port = window.location.port ? `:${window.location.port}` : ''
  const path = `/novnc/${code}/${teamName.value}-${boxName.value}`
  const url = `${protocol}://${host}${port}${path}`

  status('Connecting')
  // Ensure the screenEl is a real DOM element
  if (screenEl.value instanceof HTMLElement) {
    rfb.value = new RFB(screenEl.value, url)

    rfb.value.addEventListener('connect', () => {
      status('Connected to ' + desktopName.value)
    })
    rfb.value.addEventListener('disconnect', (e: any) => {
      if (e.detail && e.detail.clean) {
        status('Disconnected')
      } else {
        status('Something went wrong, connection is closed')
      }
    })
    rfb.value.addEventListener('desktopname', (e: any) => {
      desktopName.value = e.detail.name
    })
    rfb.value.addEventListener('securityfailure', (e: any) => {
      status('Security failure: ' + (e.detail ? e.detail.status : 'unknown'))
      console.error('noVNC security failure', e)
    })
    rfb.value.addEventListener('credentialsrequired', () => {
      status('VNC credentials required')
      console.error('noVNC credentials required')
    })
    rfb.value.addEventListener('clipboard', (e: any) => {
      // Optionally handle clipboard events
      console.log('noVNC clipboard event', e)
    })
    rfb.value.addEventListener('bell', () => {
      // Optionally handle bell events
      console.log('noVNC bell event')
    })
    rfb.value.addEventListener('capabilities', (e: any) => {
      // Optionally handle capabilities events
      console.log('noVNC capabilities', e)
    })
    rfb.value.addEventListener('fbupdate', (e: any) => {
      // Framebuffer update event
      console.log('noVNC framebuffer update', e)
    })
    rfb.value.viewOnly = false
    rfb.value.scaleViewport = true
  } else {
    status('Screen element not found')
  }
})
</script>

<template>
  <div class="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
    <h1 class="text-3xl font-bold mb-6 text-subheading">Console: {{ teamName }} / {{ boxName }}</h1>
    <div v-if="!boxName || !teamName" class="text-muted">Missing team or box parameter.</div>
    <div v-else class="flex flex-col items-center" style="min-height: 600px; width: 100%;">
      <div id="top_bar" class="w-full flex items-center justify-between bg-blue-900 text-white px-4 py-2 rounded-t">
        <div id="status">{{ statusText }}</div>
        <button id="sendCtrlAltDelButton" @click="sendCtrlAltDel" class="bg-blue-700 hover:bg-blue-600 px-3 py-1 rounded ml-2">Send CtrlAltDel</button>
      </div>
      <div id="screen" ref="screenEl" style="width: 100%; height: 600px; background: #222; border-radius: 0 0 8px 8px; overflow: hidden; display: block;"></div>
    </div>
  </div>
</template>

<style scoped>
.text-muted {
  color: #888;
}
#top_bar {
  border-bottom: 1px solid #3b5998;
}
#screen {
  min-width: 320px;
  min-height: 240px;
  width: 100%;
  height: 600px;
  background: #222;
  border-radius: 0 0 8px 8px;
  overflow: hidden;
  display: block;
}
</style>
