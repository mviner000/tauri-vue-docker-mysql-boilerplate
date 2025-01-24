<!-- src/components/SystemInfoNavbar.vue -->
<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { Badge } from '@/components/ui/badge'

type DockerStatus = 
  | 'Not Installed' 
  | 'Installing' 
  | 'Installed'
  | 'Setup Failed'

const osType = ref('')
const dockerStatus = ref<DockerStatus>('Not Installed')
const setupLogs = ref<string[]>([])

let unlistenInstallation: (() => void) | null = null

onMounted(async () => {
  try {
    osType.value = await invoke('get_os_type')
    
    const isInstalled = await invoke('is_docker_installed')
    dockerStatus.value = isInstalled ? 'Installed' : 'Not Installed'

    // Listen to Docker installation events
    unlistenInstallation = await listen('installation-stage', (event) => {
      const stage = event.payload as string
      
      // Update status based on installation stages
      switch (stage) {
        case 'DockerInstalling':
          dockerStatus.value = 'Installing'
          break
        case 'DockerInstalled':
          dockerStatus.value = 'Installed'
          break
        case 'DockerInstallFailed':
          dockerStatus.value = 'Setup Failed'
          break
        case 'SetupComplete':
          dockerStatus.value = 'Installed'
          break
      }

      // Optional: Add logging
      setupLogs.value.push(`Stage: ${stage}`)
    })
  } catch (error) {
    console.error('Setup tracking failed', error)
    dockerStatus.value = 'Setup Failed'
  }
})

onUnmounted(() => {
  if (unlistenInstallation) unlistenInstallation()
})
</script>

<template>
  <nav class="bg-gray-100 p-2 flex justify-between items-center">
    <div class="flex items-center space-x-2">
      <span>OS:</span>
      <Badge variant="secondary">{{ osType }}</Badge>
      <span>Docker:</span>
      <Badge 
        :variant="
          dockerStatus === 'Installed' ? 'default' : 
          dockerStatus === 'Installing' ? 'outline' : 
          dockerStatus === 'Setup Failed' ? 'destructive' :
          'destructive'
        "
      >
        {{ dockerStatus }}
      </Badge>
    </div>
    
    <!-- Optional: Detailed logs display -->
    <div v-if="setupLogs.length > 0" class="text-xs text-gray-600">
      <div v-for="(log, index) in setupLogs" :key="index">
        {{ log }}
      </div>
    </div>
  </nav>
</template>