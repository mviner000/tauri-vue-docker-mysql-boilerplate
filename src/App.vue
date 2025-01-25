<!-- src/App.vue -->


<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { Event, listen } from '@tauri-apps/api/event'
import { Toaster } from '@/components/ui/toast'
import NotesTable from '@/components/notes/NotesTable.vue'
import SystemInfoNavbar from '@/components/SystemInfoNavbar.vue'
import UbuntuSetupModal from './components/UbuntuSetupModal.vue'

const isUbuntuSetupRequired = ref(false)
const currentSudoPasswordRequestId = ref('')

// Handle sudo password requests from backend
const handleSudoPasswordRequest = async (event: Event<{ request_id: string }>) => {
  try {
    console.log('DEBUG: Received password request event:', event)
    
    if (!event.payload?.request_id) {
      console.error('Invalid request format - missing request_id')
      return
    }

    // Update the request ID and show modal
    currentSudoPasswordRequestId.value = event.payload.request_id
    isUbuntuSetupRequired.value = true
    
  } catch (error) {
    console.error('Error handling password request:', error)
  }
}

// Handle installation stage updates
const handleInstallationStage = async (event: Event<string>) => {
  const stage = event.payload
  if (stage === 'SetupComplete') {
    // Close modal after short delay
    setTimeout(() => {
      isUbuntuSetupRequired.value = false
    }, 5000)
  }
}

onMounted(async () => {
  try {
    // Set up event listeners
    await listen('sudo-password-request', handleSudoPasswordRequest)
    await listen('installation-stage', handleInstallationStage)

    // Check OS type on mount
    const osType = await invoke('get_os_type') as string
    if (osType === 'Linux') {
      isUbuntuSetupRequired.value = true
    }
  } catch (error) {
    console.error('Initialization error:', error)
  }
})
</script>

<template>
  <div class="min-h-screen bg-background">
    <SystemInfoNavbar />
    <Toaster />
    
    <UbuntuSetupModal
      v-model="isUbuntuSetupRequired" 
      :current-sudo-password-request-id="currentSudoPasswordRequestId"
    />
    
    <NotesTable v-if="!isUbuntuSetupRequired" />
  </div>
</template>