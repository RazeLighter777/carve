<script setup lang="ts">
import { useRoute } from 'vue-router'
import { computed, ref, onMounted, nextTick } from 'vue'
// @ts-ignore
import RFB from '@novnc/novnc/lib/rfb.js'
import { apiService } from '@/services/api'
import Dialog from '@/components/Dialog.vue'
import type { AxiosError } from 'axios'

const route = useRoute()
const boxName = computed(() => route.params.box as string || '')
const teamName = computed(() => route.params.team as string || '')

const statusText = ref('Loading')
const screenEl = ref<HTMLElement | null>(null)
const rfb = ref<any>(null)
const desktopName = ref('')
const showDialog = ref(false)
const dialogTitle = ref('')
const dialogMessage = ref('')
const dialogAction = ref<null | (() => void)>(null)
const boxCreds = ref<{username: string, password: string} | null>(null)
const boxCredsError = ref('')
const competitionName = ref('')
function status(text: string) {
  statusText.value = text
}

function openDialog(title: string, message: string, action: () => void) {
  dialogTitle.value = title
  dialogMessage.value = message
  dialogAction.value = action
  showDialog.value = true
}
function onDialogConfirm() {
  if (dialogAction.value) dialogAction.value()
  showDialog.value = false
}
function onDialogCancel() {
  showDialog.value = false
}

function sendCtrlAltDel() {
  openDialog('Send Ctrl+Alt+Del', 'Are you sure you want to send Ctrl+Alt+Del to the remote machine?', () => {
    if (rfb.value) {
      rfb.value.sendCtrlAltDel()
      status('Ctrl+Alt+Del sent')
    }
  })
}

function machineReboot() {
  openDialog('Reboot Machine', 'Are you sure you want to request a reboot of the remote machine?', () => {
    if (rfb.value && typeof rfb.value.machineReboot === 'function') {
      rfb.value.machineReboot();
      status('Reboot requested');
    }
  })
}

function machineReset() {
  openDialog('Reset Machine', 'Are you sure you want to request a reset of the remote machine?', () => {
    if (rfb.value && typeof rfb.value.machineReset === 'function') {
      rfb.value.machineReset();
      status('Reset requested');
    }
  })
}

function machineShutdown() {
  openDialog('Shutdown Machine', 'Are you sure you want to request a shutdown of the remote machine?', () => {
    if (rfb.value && typeof rfb.value.machineShutdown === 'function') {
      rfb.value.machineShutdown();
      status('Shutdown requested');
    }
  })
}

function fullscreen() {
  const el = screenEl.value as HTMLElement | null
  if (el && el.requestFullscreen) {
    el.requestFullscreen()
  } else if (el && (el as any).webkitRequestFullscreen) {
    (el as any).webkitRequestFullscreen()
  } else if (el && (el as any).msRequestFullscreen) {
    (el as any).msRequestFullscreen()
  }
}

async function fetchBoxCreds() {
  boxCreds.value = null
  boxCredsError.value = ''
  if (!boxName.value) return
  try {
    boxCreds.value = await apiService.getBoxCreds(`${boxName.value}.${teamName.value}.${competitionName.value}.hack`)
  } catch (e: any) {
    boxCredsError.value = e?.response?.data?.error || 'Could not fetch box credentials.'
  }
}

function restoreBox() {
  openDialog('Restore Box', 'Are you sure you want to restore this box to its previous state? This action cannot be undone.', async () => {
    try {
      await apiService.sendBoxRestore({ boxName: `${boxName.value}.${teamName.value}.${competitionName.value}.hack` })
      status('Box restore requested')
    } catch (e : AxiosError | any) {
      status('Failed to request box restore: ' + e?.response?.data?.error || 'Unknown error')
    }
  })
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
    //log novnc capabilities
    console.log('noVNC capabilities:', rfb.value.capabilities);
  } else {
    status('Screen element not found')
  }
  // fetch competition name
  competitionName.value = (await apiService.getCompetition()).name;
  await fetchBoxCreds()
})
</script>

<template>
  <div class="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
    <h1 class="text-3xl font-bold mb-6 text-subheading">Console: {{ teamName }} / {{ boxName }}</h1>
    <div v-if="!boxName || !teamName" class="text-muted">Missing team or box parameter.</div>
    <div v-else class="flex flex-col items-center" style="min-height: 600px; width: 100%;">
      <div v-if="boxCreds || boxCredsError" class="w-full mb-4">
        <div v-if="boxCreds" class="bg-gray-100 border border-gray-300 rounded p-3 text-center">
          <span class="font-semibold">Box Credentials:</span>
          <span class="ml-2 font-mono">{{ boxCreds.username }}</span>
          <span class="mx-1">/</span>
          <span class="font-mono">{{ boxCreds.password }}</span>
        </div>
        <div v-else-if="boxCredsError" class="text-red-600 text-center">{{ boxCredsError }}</div>
      </div>
      <div id="top_bar" class="w-full flex items-center justify-between bg-blue-900 text-white px-4 py-2 rounded-t">
        <div id="status">{{ statusText }}</div>
        <div class="flex items-center">
          <button id="sendCtrlAltDelButton" @click="sendCtrlAltDel" class="bg-blue-700 hover:bg-blue-600 px-3 py-1 rounded ml-2">Send CtrlAltDel</button>
          <button @click="machineReboot" class="bg-blue-700 hover:bg-blue-600 px-3 py-1 rounded ml-2">Reboot</button>
          <button @click="machineReset" class="bg-blue-700 hover:bg-blue-600 px-3 py-1 rounded ml-2">Reset</button>
          <button @click="machineShutdown" class="bg-blue-700 hover:bg-blue-600 px-3 py-1 rounded ml-2">Shutdown</button>
          <button @click="fullscreen" class="bg-blue-700 hover:bg-blue-600 px-3 py-1 rounded ml-2">Fullscreen</button>
          <button @click="restoreBox" class="bg-blue-700 hover:bg-blue-600 px-3 py-1 rounded ml-2">Restore Box</button>
        </div>
      </div>
      <div id="screen" ref="screenEl" style="width: 100%; height: 600px; background: #222; border-radius: 0 0 8px 8px; overflow: hidden; display: block;"></div>
    </div>
    <Dialog :visible="showDialog" :title="dialogTitle" :message="dialogMessage" @confirm="onDialogConfirm" @cancel="onDialogCancel" />
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
