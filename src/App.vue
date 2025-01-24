<!-- src/App.vue -->
<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { Toaster } from '@/components/ui/toast'
import NotesTable from '@/components/notes/NotesTable.vue'
import UbuntuSetupModal from '@/components/UbuntuSetupModal.vue'
import SystemInfoNavbar from '@/components/SystemInfoNavbar.vue'

const isUbuntuSetupRequired = ref(false)
const isSetupComplete = ref(false)

onMounted(async () => {
  try {
    const osType = await invoke('get_os_type') as string
    if (osType === 'Linux') {
      isUbuntuSetupRequired.value = true
    }
  } catch (error) {
    console.error('OS detection failed', error)
  }
})
</script>

<template>
  <div>
    <SystemInfoNavbar />
    <Toaster />
    <UbuntuSetupModal 
      v-model="isUbuntuSetupRequired" 
      v-if="isUbuntuSetupRequired" 
    />
    <NotesTable v-if="!isUbuntuSetupRequired" />
  </div>
</template>