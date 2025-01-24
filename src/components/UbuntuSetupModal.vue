<!-- src/components/UbuntuSetupModal.vue -->
<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick } from 'vue'
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

const mysqlContainerLogs = ref<string[]>([])
const logContainerRef = ref<HTMLDivElement | null>(null)

let unlistenStage: (() => void) | null = null
let unlistenLogs: (() => void) | null = null

onMounted(async () => {
  try {
    // Listen to installation stages
    unlistenStage = await listen('installation-stage', (event) => {
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

    // Listen to MySQL container logs specifically during MySQL container stages
    unlistenLogs = await listen('mysql-container-log', (event) => {
      const logMessage = event.payload as string
      mysqlContainerLogs.value.push(logMessage)
      
      // Auto-scroll to bottom
      nextTick(() => {
        if (logContainerRef.value) {
          logContainerRef.value.scrollTop = logContainerRef.value.scrollHeight
        }
      })
    })
  } catch (error) {
    console.error('Failed to listen to events:', error)
    errorMessage.value = 'Event listening failed'
  }
})

onUnmounted(() => {
  if (unlistenStage) unlistenStage()
  if (unlistenLogs) unlistenLogs()
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
  
          <!-- MySQL Container Logs Display -->
          <div 
            v-if="currentStage === 'StartingMySQLContainer' || 
                   currentStage === 'MySQLContainerStarted' || 
                   currentStage === 'MySQLSetupFailed'"
            class="mt-4"
          >
            <h3 class="text-sm font-medium mb-2">MySQL Container Logs</h3>
            <div 
              ref="logContainerRef"
              class="border rounded-md p-2 max-h-48 overflow-y-auto bg-muted/50 font-mono text-xs"
            >
              <div v-for="(log, index) in mysqlContainerLogs" :key="index" class="whitespace-pre-wrap">
                {{ log }}
              </div>
            </div>
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