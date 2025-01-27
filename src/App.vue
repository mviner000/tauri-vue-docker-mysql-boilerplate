<!-- src/App.vue -->
<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { Event, listen } from '@tauri-apps/api/event'
import { Toaster } from '@/components/ui/toast'
import NotesTable from '@/components/notes/NotesTable.vue'
import SystemInfoNavbar from '@/components/SystemInfoNavbar.vue'
import UbuntuSetupModal from './components/UbuntuSetupModal.vue'
import WindowsSetupModal from './components/WindowsSetupModal.vue'

interface SetupState {
  isSetupRequired: boolean
  currentSudoPasswordRequestId: string
  osType: string | null
}

const setupState = ref<SetupState>({
  isSetupRequired: false,
  currentSudoPasswordRequestId: '',
  osType: null
})

// Handle sudo password requests from backend (Ubuntu-specific)
const handleSudoPasswordRequest = async (event: Event<{ request_id: string }>) => {
  try {
    console.log('DEBUG: Received password request event:', event)
    
    if (!event.payload?.request_id) {
      console.error('Invalid request format - missing request_id')
      return
    }

    setupState.value.currentSudoPasswordRequestId = event.payload.request_id
    setupState.value.isSetupRequired = true
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
      setupState.value.isSetupRequired = false
    }, 5000)
  }
}

// Handle modal state changes
const handleModalOpenChange = (open: boolean) => {
  setupState.value.isSetupRequired = open
}

// Initialize system based on OS type
const initializeSystem = async () => {
  try {
    const osType = await invoke('get_os_type') as string
    setupState.value.osType = osType

    // Set up event listeners for both OS types
    await listen('installation-stage', handleInstallationStage)

    switch (osType) {
      case 'Linux':
        await listen('sudo-password-request', handleSudoPasswordRequest)
        setupState.value.isSetupRequired = true
        break
        
      case 'Windows':
        const isDockerInstalled = await invoke('is_docker_installed') as boolean
        if (!isDockerInstalled) {
          setupState.value.isSetupRequired = true
        }
        break
        
      default:
        console.log('Unsupported OS:', osType)
    }
  } catch (error) {
    console.error('System initialization error:', error)
  }
}

onMounted(async () => {
  await initializeSystem()
})
</script>

<template>
  <div class="min-h-screen bg-background">
    <SystemInfoNavbar />
    <Toaster />
    
    <!-- Dynamic Modal Selection Based on OS -->
    <template v-if="setupState.isSetupRequired">
      <WindowsSetupModal
        v-if="setupState.osType === 'Windows'"
        :open="setupState.isSetupRequired"
        :onOpenChange="handleModalOpenChange"
      />
      
      <UbuntuSetupModal
        v-if="setupState.osType === 'Linux'"
        v-model="setupState.isSetupRequired"
        :current-sudo-password-request-id="setupState.currentSudoPasswordRequestId"
      />
    </template>
    
    <NotesTable v-if="!setupState.isSetupRequired" />
  </div>
</template>