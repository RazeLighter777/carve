<template>
  <div class="max-w-4xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
    <h1 class="text-3xl font-bold mb-6 text-subheading">Support Tickets</h1>
    
    <!-- Loading state -->
    <div v-if="loading" class="text-muted">Loading tickets...</div>
    
    <!-- Error state -->
    <div v-if="error" class="bg-red-100 dark:bg-red-900/30 border border-red-400 dark:border-red-600 text-red-700 dark:text-red-300 px-4 py-3 rounded mb-4">
      {{ error }}
    </div>

    <!-- Create new ticket section -->
    <div class="bg-white dark:bg-gray-800 rounded-lg shadow-md p-6 mb-6">
      <h2 class="text-xl font-semibold mb-4 text-gray-900 dark:text-gray-100">Create New Ticket</h2>
      <form @submit.prevent="createTicket" class="space-y-4">
        <div>
          <label for="newTicketSubject" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
            Subject
          </label>
          <input
            id="newTicketSubject"
            v-model="newTicketSubject"
            type="text"
            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 placeholder-gray-400 dark:placeholder-gray-500 rounded-md shadow-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
            placeholder="Brief description of your issue..."
            required
          />
        </div>
        <div>
          <label for="newTicketMessage" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
            Message
          </label>
          <textarea
            id="newTicketMessage"
            v-model="newTicketMessage"
            class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 placeholder-gray-400 dark:placeholder-gray-500 rounded-md shadow-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
            rows="4"
            placeholder="Describe your issue or question..."
            required
          ></textarea>
        </div>
        <button
          type="submit"
          :disabled="creatingTicket || !newTicketMessage.trim() || !newTicketSubject.trim()"
          class="bg-blue-600 dark:bg-blue-700 text-white px-4 py-2 rounded-md hover:bg-blue-700 dark:hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {{ creatingTicket ? 'Creating...' : 'Create Ticket' }}
        </button>
      </form>
    </div>

    <!-- Tickets list and viewer -->
    <div v-if="!loading" class="bg-white dark:bg-gray-800 rounded-lg shadow-md p-6">
      <!-- Ticket selector -->
      <div class="mb-6">
        <label for="ticketSelect" class="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
          Select Ticket
        </label>
        <select
          id="ticketSelect"
          v-model="selectedTicketId"
          @change="loadSelectedTicket"
          class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 rounded-md shadow-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
        >
          <option value="">Select a ticket...</option>
          <option
            v-for="ticketResponse in tickets"
            :key="ticketResponse.ticketId"
            :value="ticketResponse.ticketId"
          >
            #{{ ticketResponse.ticketId }}: {{ ticketResponse.ticket.subject }} ({{ ticketResponse.ticket.team_name }}) - {{ formatDate(ticketResponse.ticket.date) }}
          </option>
        </select>
      </div>

      <!-- No tickets message -->
      <div v-if="tickets.length === 0" class="text-gray-500 dark:text-gray-400 text-center py-8">
        No support tickets found. Create your first ticket above!
      </div>

      <!-- Selected ticket viewer -->
      <div v-if="selectedTicket" class="space-y-6">
        <div class="border-b dark:border-gray-700 pb-4">
          <div class="flex justify-between items-start">
            <div>
              <h3 class="text-lg font-semibold text-gray-900 dark:text-gray-100">{{ selectedTicket.ticket.subject }}</h3>
              <p class="text-sm text-gray-500 dark:text-gray-400">Ticket #{{ selectedTicket.ticketId }}</p>
              <p class="text-sm text-gray-600 dark:text-gray-400">
                Created: {{ formatDate(selectedTicket.ticket.date) }} by {{ selectedTicket.ticket.team_name }}
              </p>
              <div class="mt-2">
                <span 
                  :class="{
                    'bg-green-100 dark:bg-green-900/30 text-green-800 dark:text-green-200': selectedTicket.ticket.state === 'Open',
                    'bg-gray-100 dark:bg-gray-700 text-gray-800 dark:text-gray-200': selectedTicket.ticket.state === 'Closed'
                  }"
                  class="px-2 py-1 rounded-full text-xs font-medium"
                >
                  {{ selectedTicket.ticket.state }}
                </span>
              </div>
            </div>
            
            <!-- Admin status controls -->
            <div v-if="isAdmin" class="flex space-x-2">
              <button
                v-if="selectedTicket.ticket.state === 'Closed'"
                @click="updateTicketStatus('open')"
                :disabled="updatingStatus"
                class="px-3 py-1 bg-green-600 dark:bg-green-700 text-white text-sm rounded-md hover:bg-green-700 dark:hover:bg-green-600 disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {{ updatingStatus ? 'Updating...' : 'Reopen' }}
              </button>
              <button
                v-if="selectedTicket.ticket.state === 'Open'"
                @click="updateTicketStatus('closed')"
                :disabled="updatingStatus"
                class="px-3 py-1 bg-red-600 dark:bg-red-700 text-white text-sm rounded-md hover:bg-red-700 dark:hover:bg-red-600 disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {{ updatingStatus ? 'Updating...' : 'Close' }}
              </button>
            </div>
          </div>
        </div>

        <!-- Messages -->
        <div class="space-y-4 max-h-96 overflow-y-auto">
          <div
            v-for="(message, index) in selectedTicket.ticket.messages"
            :key="index"
            :class="{
              'bg-blue-50 dark:bg-blue-900/30 border-l-4 border-blue-500': message.sender === 'team',
              'bg-green-50 dark:bg-green-900/30 border-l-4 border-green-500': message.sender === 'admin'
            }"
            class="p-4 rounded-md"
          >
            <div class="flex justify-between items-start mb-2">
              <span class="font-medium text-sm text-gray-900 dark:text-gray-100">
                {{ message.sender === 'team' ? 'Team' : 'Administrator' }}
              </span>
              <span class="text-xs text-gray-500 dark:text-gray-400">
                {{ formatDate(message.timestamp) }}
              </span>
            </div>
            <p class="text-gray-800 dark:text-gray-200 whitespace-pre-wrap">{{ message.message }}</p>
          </div>
        </div>

        <!-- Reply form -->
        <div class="border-t dark:border-gray-700 pt-4">
          <h4 class="text-md font-semibold mb-3 text-gray-900 dark:text-gray-100">Add Reply</h4>
          <form @submit.prevent="addReply" class="space-y-4">
            <div>
              <textarea
                v-model="replyMessage"
                class="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 placeholder-gray-400 dark:placeholder-gray-500 rounded-md shadow-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                rows="3"
                placeholder="Type your reply..."
                required
              ></textarea>
            </div>
            <button
              type="submit"
              :disabled="addingReply || !replyMessage.trim()"
              class="bg-green-600 dark:bg-green-700 text-white px-4 py-2 rounded-md hover:bg-green-700 dark:hover:bg-green-600 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {{ addingReply ? 'Sending...' : 'Send Reply' }}
            </button>
          </form>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import apiService from '@/services/api'
import { cookieUtils } from '@/utils/cookies'
import type { 
  SupportTicketsResponse, 
  SupportTicketResponse, 
  CreateSupportTicketRequest,
  AddSupportTicketMessageRequest,
  UpdateSupportTicketStatusRequest,
  SupportTicketState
} from '@/types'

// Reactive state
const loading = ref(true)
const error = ref('')
const tickets = ref<SupportTicketsResponse['tickets']>([])
const selectedTicketId = ref<number | ''>('')
const selectedTicket = ref<SupportTicketResponse | null>(null)

// New ticket form
const newTicketMessage = ref('')
const newTicketSubject = ref('')
const creatingTicket = ref(false)

// Reply form
const replyMessage = ref('')
const addingReply = ref(false)

// Admin functionality
const userInfo = ref<any>(null)
const isAdmin = computed(() => userInfo.value?.is_admin || false)
const updatingStatus = ref(false)

onMounted(async () => {
  userInfo.value = cookieUtils.getUserInfo()
  await loadTickets()
})

// Load all tickets
const loadTickets = async () => {
  try {
    loading.value = true
    error.value = ''
    const response = await apiService.getSupportTickets()
    tickets.value = response.tickets
  } catch (err) {
    console.error('Failed to load tickets:', err)
    error.value = 'Failed to load support tickets'
  } finally {
    loading.value = false
  }
}

// Load selected ticket details
const loadSelectedTicket = async () => {
  if (!selectedTicketId.value) {
    selectedTicket.value = null
    return
  }

  try {
    const response = await apiService.getSupportTicket(selectedTicketId.value as number)
    selectedTicket.value = response
  } catch (err) {
    console.error('Failed to load ticket details:', err)
    error.value = 'Failed to load ticket details'
  }
}

// Create new ticket
const createTicket = async () => {
  if (!newTicketMessage.value.trim() || !newTicketSubject.value.trim()) return

  try {
    creatingTicket.value = true
    error.value = ''
    
    const request: CreateSupportTicketRequest = {
      message: newTicketMessage.value.trim(),
      subject: newTicketSubject.value.trim()
    }
    
    const response = await apiService.createSupportTicket(request)
    
    // Clear form
    newTicketMessage.value = ''
    newTicketSubject.value = ''
    
    // Reload tickets list
    await loadTickets()
    
    // Auto-select the new ticket
    selectedTicketId.value = response.ticketId
    await loadSelectedTicket()
    
  } catch (err) {
    console.error('Failed to create ticket:', err)
    error.value = 'Failed to create support ticket'
  } finally {
    creatingTicket.value = false
  }
}

// Add reply to selected ticket
const addReply = async () => {
  if (!selectedTicketId.value || !replyMessage.value.trim()) return

  try {
    addingReply.value = true
    error.value = ''
    
    const request: AddSupportTicketMessageRequest = {
      message: replyMessage.value.trim()
    }
    
    await apiService.addSupportTicketMessage(selectedTicketId.value as number, request)
    
    // Clear reply form
    replyMessage.value = ''
    
    // Reload the selected ticket to show the new message
    await loadSelectedTicket()
    
  } catch (err) {
    console.error('Failed to add reply:', err)
    error.value = 'Failed to add reply to ticket'
  } finally {
    addingReply.value = false
  }
}

// Format date for display
const formatDate = (dateString: string): string => {
  const date = new Date(dateString)
  return date.toLocaleString()
}

// Admin: Update ticket status
const updateTicketStatus = async (status: string) => {
  if (!selectedTicketId.value || !isAdmin.value) return

  try {
    updatingStatus.value = true
    error.value = ''
    
    const request: UpdateSupportTicketStatusRequest = {
      status: status
    }
    
    await apiService.updateSupportTicketStatus(selectedTicketId.value as number, request)
    
    // Reload the selected ticket to show the updated status
    await loadSelectedTicket()
    
  } catch (err) {
    console.error('Failed to update ticket status:', err)
    error.value = 'Failed to update ticket status'
  } finally {
    updatingStatus.value = false
  }
}
</script>

<style scoped>
/* Custom scrollbar for messages */
.overflow-y-auto::-webkit-scrollbar {
  width: 6px;
}

.overflow-y-auto::-webkit-scrollbar-track {
  background: #f1f1f1;
  border-radius: 3px;
}

.overflow-y-auto::-webkit-scrollbar-thumb {
  background: #c1c1c1;
  border-radius: 3px;
}

.overflow-y-auto::-webkit-scrollbar-thumb:hover {
  background: #a8a8a8;
}

/* Dark mode scrollbar */
:global(.dark) .overflow-y-auto::-webkit-scrollbar-track {
  background: #374151;
}

:global(.dark) .overflow-y-auto::-webkit-scrollbar-thumb {
  background: #6b7280;
}

:global(.dark) .overflow-y-auto::-webkit-scrollbar-thumb:hover {
  background: #9ca3af;
}
</style>
