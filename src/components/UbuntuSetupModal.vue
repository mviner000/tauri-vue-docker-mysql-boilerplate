<!-- src/components/UbuntuSetupModal.vue -->
<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick, watch } from 'vue'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Progress } from '@/components/ui/progress'
import { listen } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'

type InstallationStage = 
  | 'NotStarted'
  | 'AwaitingInstallationStart'
  | 'AwaitingSudoPassword'
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
  },
  currentSudoPasswordRequestId: {
    type: String,
    required: true
  }
})

const emit = defineEmits(['update:modelValue', 'password-submitted'])

const currentStage = ref<InstallationStage>('NotStarted')
const setupProgress = ref(0)
const errorMessage = ref('')
const password = ref('')
const mysqlContainerLogs = ref<string[]>([])
const dockerInstallLogs = ref<string[]>([])
const logContainerRef = ref<HTMLDivElement | null>(null)
const dockerLogContainerRef = ref<HTMLDivElement | null>(null)

const stageDescriptions: Record<InstallationStage, string> = {
  NotStarted: 'Initializing system setup...',
  AwaitingInstallationStart: 'Ready to begin system setup',
  AwaitingSudoPassword: 'Please enter your sudo password to continue setup.',
  CheckingDocker: 'Checking Docker installation...',
  DockerNotInstalled: 'Docker not found. Preparing installation...',
  DockerInstalling: 'Installing Docker. Please follow terminal instructions.',
  DockerInstallFailed: 'Docker installation failed.',
  DockerInstalled: 'Docker successfully installed.',
  PreparingMySQLContainer: 'Preparing MySQL container...',
  StartingMySQLContainer: 'Starting MySQL container...',
  MySQLContainerStarted: 'MySQL container started successfully. Creating database...',
  MySQLSetupFailed: 'MySQL container setup failed.',
  SetupComplete: 'System setup completed! Database and tables created successfully.'
}

const calculateProgress = (stage: InstallationStage): number => {
  const stages: InstallationStage[] = [
    'NotStarted', 'AwaitingSudoPassword', 'CheckingDocker', 
    'DockerNotInstalled', 'DockerInstalling', 'DockerInstalled',
    'PreparingMySQLContainer', 'StartingMySQLContainer', 
    'MySQLContainerStarted', 'SetupComplete'
  ]
  const index = stages.indexOf(stage);
  return Math.round((index / (stages.length - 1)) * 100);
}

let unlistenStage: (() => void) | null = null
let unlistenLogs: (() => void) | null = null
let unlistenDockerLogs: (() => void) | null = null

const currentRequestId = ref('')

onMounted(async () => {
  try {
    currentStage.value = 'AwaitingInstallationStart';

    const unsubscribe = await listen('sudo-password-request', (event: any) => {
      if (event.payload?.request_id) {
        currentRequestId.value = event.payload.request_id;
        currentStage.value = 'AwaitingSudoPassword';
      } else {
        console.error('Invalid sudo password request payload:', event.payload);
      }
    });

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

    unlistenLogs = await listen('mysql-container-log', (event) => {
      const logMessage = event.payload as string
      mysqlContainerLogs.value.push(logMessage)
      
      nextTick(() => {
        if (logContainerRef.value) {
          logContainerRef.value.scrollTop = logContainerRef.value.scrollHeight
        }
      })
    })

    // New listener for Docker installation logs
    unlistenDockerLogs = await listen('docker-install-log', (event) => {
      const logMessage = event.payload as string
      dockerInstallLogs.value.push(logMessage)
      
      nextTick(() => {
        if (dockerLogContainerRef.value) {
          dockerLogContainerRef.value.scrollTop = dockerLogContainerRef.value.scrollHeight
        }
      })
    })
  } catch (error) {
    console.error('Failed to listen to events:', error)
    errorMessage.value = 'Event listening failed'
  }
})

onUnmounted(() => {
  if (unlistenStage) unlistenStage();
  if (unlistenLogs) unlistenLogs();
  if (unlistenDockerLogs) unlistenDockerLogs();
});

const startInstallation = async () => {
  try {
    currentStage.value = 'CheckingDocker';
    await invoke('start_ubuntu_setup');
  } catch (error) {
    console.error('Failed to start installation:', error);
    errorMessage.value = 'Failed to start installation';
  }
};

const handlePasswordSubmit = async () => {
  if (password.value.trim()) {
    try {
      if (!props.currentSudoPasswordRequestId) {
        errorMessage.value = 'No active password request. Please retry.';
        return;
      }
      
      await invoke('respond_to_sudo_password_request', { 
        requestId: props.currentSudoPasswordRequestId,
        password: password.value 
      });
      
      currentStage.value = 'CheckingDocker';
      errorMessage.value = '';
    } catch (error) {
      console.error('Password submission failed:', error);
      errorMessage.value = 'Failed to submit password. Please try again.';
    }
  } else {
    errorMessage.value = 'Please enter a password';
  }
}

const handleClose = () => {
  emit('update:modelValue', false)
}

// Update state handling to ensure immediate updates
watch(currentStage, (newStage) => {
  setupProgress.value = calculateProgress(newStage)
  
  if (newStage === 'DockerInstallFailed') {
    errorMessage.value = `Docker installation failed. Check logs below.`
  }
  
  // Force UI update for immediate state changes
  nextTick(() => {
    if (dockerLogContainerRef.value) {
      dockerLogContainerRef.value.scrollTop = dockerLogContainerRef.value.scrollHeight
    }
  })
})
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

        <!-- Start Installation Button -->
        <div v-if="currentStage === 'AwaitingInstallationStart'" class="flex justify-center">
          <Button @click="startInstallation">
            Start Installation
          </Button>
        </div>

        <!-- Sudo Password Input -->
        <div v-if="currentStage === 'AwaitingSudoPassword'" class="grid gap-4 py-4">
          <div class="grid grid-cols-4 items-center gap-4">
            <Label for="sudo-password" class="text-right">
              Password
            </Label>
            <Input 
              id="sudo-password" 
              type="password" 
              v-model="password" 
              @keyup.enter="handlePasswordSubmit"
              class="col-span-3"
            />
          </div>
        </div>

        <!-- Docker Installation Logs Display -->
        <div 
          v-if="currentStage === 'DockerNotInstalled' || 
                 currentStage === 'DockerInstalling' || 
                 currentStage === 'DockerInstallFailed'"
          class="mt-4"
        >
          <h3 class="text-sm font-medium mb-2">Docker Installation Logs</h3>
          <div 
            ref="dockerLogContainerRef"
            class="border rounded-md p-2 max-h-48 overflow-y-auto bg-muted/50 font-mono text-xs"
          >
            <div v-for="(log, index) in dockerInstallLogs" :key="index" class="whitespace-pre-wrap">
              {{ log }}
            </div>
          </div>
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

      <DialogFooter>
        <Button 
          v-if="currentStage === 'AwaitingSudoPassword'"
          type="submit" 
          @click="handlePasswordSubmit"
        >
          Submit
        </Button>
        
        <!-- Success Button -->
        <Button 
          v-if="currentStage === 'SetupComplete'"
          @click="handleClose"
        >
          Finished
        </Button>
        
        <!-- Cancel/Close Button -->
        <Button 
          v-else
          variant="outline" 
          @click="handleClose" 
          :disabled="currentStage !== 'DockerInstallFailed' && currentStage !== 'AwaitingSudoPassword'"
        >
          Cancel
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>