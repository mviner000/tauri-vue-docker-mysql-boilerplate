<!-- src/App.vue -->

<script setup lang="ts">
import { Toaster } from '@/components/ui/toast'
import NotesTable from '@/components/notes/NotesTable.vue'
</script>

<template>
  <div>
    <Toaster />
    <NotesTable />
  </div>
</template>