<!-- src/components/UbuntuSetupModal.vue -->

<!-- src/components/WindowsSetupModal.vue -->
<script setup lang="ts">
import { ref, onMounted, watchEffect } from 'vue';
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Progress } from '@/components/ui/progress';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

type InstallationStage = 
  | 'NotStarted'
  | 'SystemCheckInProgress'
  | 'SystemCheckComplete'
  | 'CheckingDocker'
  | 'DockerNotInstalled'
  | 'DockerInstalling'
  | 'DockerInstallFailed'
  | 'DockerInstalled'
  | 'StartingMySQLContainer'
  | 'MySQLContainerStarted'
  | 'MySQLSetupFailed'
  | 'SetupComplete';

const stageDescriptions: Record<InstallationStage, string> = {
  NotStarted: 'Initializing system check...',
  SystemCheckInProgress: 'Verifying system requirements...',
  SystemCheckComplete: 'System requirements verified.',
  CheckingDocker: 'Checking Docker Desktop installation...',
  DockerNotInstalled: 'Docker Desktop is required but not installed.',
  DockerInstalling: 'Installing Docker Desktop...',
  DockerInstallFailed: 'Docker Desktop installation failed.',
  DockerInstalled: 'Docker Desktop installed successfully.',
  StartingMySQLContainer: 'Setting up MySQL container...',
  MySQLContainerStarted: 'MySQL container started successfully.',
  MySQLSetupFailed: 'MySQL setup failed.',
  SetupComplete: 'System setup completed successfully!'
};

const calculateProgress = (stage: InstallationStage): number => {
  const stages: InstallationStage[] = [
    'NotStarted',
    'SystemCheckInProgress',
    'SystemCheckComplete',
    'CheckingDocker',
    'DockerNotInstalled',
    'DockerInstalling',
    'DockerInstalled',
    'StartingMySQLContainer',
    'MySQLContainerStarted',
    'SetupComplete'
  ];
  const index = stages.indexOf(stage);
  return Math.round((index / (stages.length - 1)) * 100);
};

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

const props = defineProps<Props>();

const currentStage = ref<InstallationStage>('NotStarted');
const setupProgress = ref(0);
const errorMessage = ref('');
const dockerLogs = ref<string[]>([]);
const systemRequirements = ref({
  isWindows: false,
  isCompatibleVersion: false,
  checkComplete: false
});

const checkSystemRequirements = async () => {
  try {
    currentStage.value = 'SystemCheckInProgress';
    setupProgress.value = calculateProgress('SystemCheckInProgress');

    // Check if running on Windows
    const osType = await invoke('get_os_type');
    systemRequirements.value.isWindows = osType === 'Windows';

    // Get detailed OS information
    const osDetails = await invoke('get_os_details');
    console.log('OS Details:', osDetails);

    // Check Windows version compatibility
    if (systemRequirements.value.isWindows) {
    const isCompatible = await invoke<boolean>('is_windows');
    systemRequirements.value.isCompatibleVersion = isCompatible;
    }

    systemRequirements.value.checkComplete = true;
    currentStage.value = 'SystemCheckComplete';
    setupProgress.value = calculateProgress('SystemCheckComplete');

    if (!systemRequirements.value.isWindows || !systemRequirements.value.isCompatibleVersion) {
      errorMessage.value = 'Your system does not meet the minimum requirements.';
      return false;
    }

    return true;
  } catch (error) {
    console.error('System check failed:', error);
    errorMessage.value = `System check failed: ${error}`;
    return false;
  }
};

const startSetup = async () => {
  try {
    const isSystemCompatible = await checkSystemRequirements();
    if (!isSystemCompatible) {
      return;
    }

    currentStage.value = 'CheckingDocker';
    setupProgress.value = calculateProgress('CheckingDocker');
    
    await invoke('start_system_setup');
  } catch (error) {
    console.error('Setup failed:', error);
    errorMessage.value = `Setup failed: ${error}`;
    currentStage.value = 'DockerInstallFailed';
  }
};

onMounted(async () => {
  // Start system check immediately when modal opens
  if (props.open) {
    await checkSystemRequirements();
  }

  await listen('installation-stage', (event: any) => {
    const stage = event.payload as InstallationStage;
    currentStage.value = stage;
    setupProgress.value = calculateProgress(stage);
  });

  await listen('docker-install-log', (event: any) => {
    const logMessage = event.payload as string;
    dockerLogs.value.push(logMessage);
  });
});

// Watch for modal open state changes
watchEffect(() => {
  if (props.open && currentStage.value === 'NotStarted') {
    checkSystemRequirements();
  }
});
</script>

<template>
  <Dialog :open="open" @update:open="onOpenChange">
    <DialogContent class="max-w-md">
      <DialogHeader>
        <DialogTitle>Windows System Setup</DialogTitle>
        <DialogDescription>
          {{ stageDescriptions[currentStage] }}
        </DialogDescription>
      </DialogHeader>

      <div class="space-y-4">
        <Progress :value="setupProgress" />

        <div v-if="errorMessage" class="text-red-500 text-sm">
          {{ errorMessage }}
        </div>

        <div v-if="systemRequirements.checkComplete && !errorMessage" class="space-y-2 text-sm">
          <div class="flex items-center">
            <span class="mr-2">✓</span>
            <span>Windows OS detected</span>
          </div>
          <div class="flex items-center">
            <span class="mr-2">✓</span>
            <span>Compatible Windows version</span>
          </div>
        </div>

        <div v-if="currentStage === 'SystemCheckComplete' && !errorMessage" class="text-center">
          <Button @click="startSetup">
            Continue Setup
          </Button>
        </div>

        <div v-if="['DockerInstalling', 'DockerInstallFailed'].includes(currentStage)" class="mt-4">
          <h3 class="text-sm font-medium mb-2">Installation Logs</h3>
          <div class="border rounded-md p-2 max-h-48 overflow-y-auto bg-muted/50 font-mono text-xs">
            <div v-for="(log, index) in dockerLogs" :key="index" class="whitespace-pre-wrap">
              {{ log }}
            </div>
          </div>
        </div>
      </div>

      <DialogFooter>
        <Button
          variant="outline"
          @click="() => onOpenChange(false)"
          :disabled="currentStage !== 'DockerInstallFailed' && 
                    currentStage !== 'SetupComplete' &&
                    currentStage !== 'SystemCheckComplete'"
        >
          {{ currentStage === 'SetupComplete' ? 'Close' : 'Cancel' }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>