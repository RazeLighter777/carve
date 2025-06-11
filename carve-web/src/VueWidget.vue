<template>
  <div class="vue-widget">
    <h2>Vue Widget</h2>
    <p>This is a Vue component embedded in the React app.</p>
    <button @click="fetchData">Fetch Vue Data</button>
    <div v-if="data" class="vue-data">{{ data }}</div>
  </div>
</template>

<script setup>
import { ref } from 'vue'
const data = ref(null)
async function fetchData() {
  const res = await fetch('/api/v1/vue-demo')
  data.value = await res.text()
}
</script>

<style scoped>
.vue-widget {
  background: #23272f;
  color: #fff;
  border-radius: 12px;
  padding: 1.5rem;
  margin: 1rem 0;
  box-shadow: 0 2px 12px #000a;
}
.vue-widget button {
  background: #3b82f6;
  color: #fff;
  border: none;
  border-radius: 6px;
  padding: 0.5rem 1rem;
  margin-top: 1rem;
  cursor: pointer;
  transition: background 0.2s;
}
.vue-widget button:hover {
  background: #2563eb;
}
.vue-data {
  margin-top: 1rem;
  font-weight: bold;
}
</style>
