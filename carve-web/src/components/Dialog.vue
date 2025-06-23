<template>
  <div v-if="visible" class="fixed inset-0 z-50 flex items-center justify-center bg-black bg-opacity-40">
    <div class="bg-white rounded-lg shadow-lg max-w-sm w-full p-6">
      <div class="mb-4 text-lg font-semibold text-gray-900">{{ title }}</div>
      <div class="mb-6 text-gray-700">{{ message }}</div>
      <div class="flex justify-end space-x-2">
        <button @click="$emit('cancel')" class="px-4 py-2 rounded bg-gray-200 hover:bg-gray-300 text-gray-800">Cancel</button>
        <button @click="$emit('confirm')" class="px-4 py-2 rounded bg-blue-700 hover:bg-blue-800 text-white">Confirm</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  visible: boolean
  title?: string
  message: string
}>()
</script>

<style scoped>
</style>
