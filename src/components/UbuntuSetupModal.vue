<!-- src/components/UbuntuSetupModal.vue -->
<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription } from '@/components/ui/dialog'
import { Progress } from '@/components/ui/progress'
import { listen } from '@tauri-apps/api/event'

// Match the Rust InstallationStage enum
type InstallationStage = 
  | 'NotStarted'
  | 'CheckingDocker'
  | 'DockerNotInstalled'
  | 'DockerInstalling'
  | 'DockerInstallFailed'
  | 'DockerInstalled'
  | 'PreparingMySQLContainer'
  | 'StartingMySQLContainer'
  | 'MySQLContainerStarted'
  | 'MySQLSetupFailed'
  | 'SetupComplete'

const props = defineProps({
  modelValue: {
    type: Boolean,
    required: true
  }
})

const emit = defineEmits(['update:modelValue'])

const currentStage = ref<InstallationStage>('NotStarted')
const setupProgress = ref(0)
const errorMessage = ref('')

const stageDescriptions: Record<InstallationStage, string> = {
  NotStarted: 'Initializing system setup...',
  CheckingDocker: 'Checking Docker installation...',
  DockerNotInstalled: 'Docker not found. Preparing installation...',
  DockerInstalling: 'Installing Docker. Please follow terminal instructions.',
  DockerInstallFailed: 'Docker installation failed.',
  DockerInstalled: 'Docker successfully installed.',
  PreparingMySQLContainer: 'Preparing MySQL container...',
  StartingMySQLContainer: 'Starting MySQL container...',
  MySQLContainerStarted: 'MySQL container started successfully.',
  MySQLSetupFailed: 'MySQL container setup failed.',
  SetupComplete: 'System setup completed!'
}

const calculateProgress = (stage: InstallationStage): number => {
  const stages: InstallationStage[] = [
    'NotStarted', 'CheckingDocker', 'DockerNotInstalled', 
    'DockerInstalling', 'DockerInstalled', 
    'PreparingMySQLContainer', 'StartingMySQLContainer', 
    'MySQLContainerStarted', 'SetupComplete'
  ]
  const index = stages.indexOf(stage)
  return index > 0 ? Math.min(100, (index / (stages.length - 1)) * 100) : 0
}

let unlisten: (() => void) | null = null

onMounted(async () => {
  try {
    unlisten = await listen('installation-stage', (event) => {
      const stage = event.payload as InstallationStage
      currentStage.value = stage
      setupProgress.value = calculateProgress(stage)

      if (stage === 'DockerInstallFailed' || stage === 'MySQLSetupFailed') {
        errorMessage.value = `Setup failed at stage: ${stage}`
      }

      if (stage === 'SetupComplete') {
        emit('update:modelValue', false)
      }
    })
  } catch (error) {
    console.error('Failed to listen to installation events:', error)
    errorMessage.value = 'Event listening failed'
  }
})

onUnmounted(() => {
  if (unlisten) unlisten()
})

const handleClose = () => {
  emit('update:modelValue', false)
}
</script>

<template>
  <Dialog :open="modelValue" @update:open="handleClose">
    <DialogContent>
      <DialogHeader>
        <DialogTitle>Ubuntu System Setup</DialogTitle>
        <DialogDescription>
          {{ stageDescriptions[currentStage] }}
        </DialogDescription>
      </DialogHeader>
      
      <div class="space-y-4">
        <Progress :value="setupProgress" />
        
        <div v-if="errorMessage" class="text-red-500">
          {{ errorMessage }}
        </div>
      </div>

      <div class="flex justify-end">
        <Button 
          variant="outline" 
          @click="handleClose" 
          :disabled="currentStage !== 'SetupComplete' && currentStage !== 'DockerInstallFailed'"
        >
          {{ currentStage === 'SetupComplete' ? 'Close' : 'Cancel' }}
        </Button>
      </div>
    </DialogContent>
  </Dialog>
</template>