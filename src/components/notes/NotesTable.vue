<!-- src/components/notes/NotesTable.vue -->

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { Button } from '@/components/ui/button'
import { 
  Table, 
  TableBody, 
  TableCaption, 
  TableCell, 
  TableHead, 
  TableHeader, 
  TableRow 
} from '@/components/ui/table'
import { 
  Dialog, 
  DialogContent, 
  DialogHeader, 
  DialogTitle, 
  DialogTrigger 
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Textarea } from '@/components/ui/textarea'
import { NoteService, Note } from '@/lib/notes'

const notes = ref<Note[]>([])
const selectedNote = ref<Note | null>(null)
const isEditDialogOpen = ref(false)
const isCreateDialogOpen = ref(false)

const newNote = ref<Note>({
  title: '',
  content: null
})

// Fetch notes on component mount
onMounted(async () => {
  try {
    notes.value = await NoteService.getAllNotes()
  } catch (error) {
    // Error handling is done in the service
  }
})

// Create a new note
async function handleCreateNote() {
  try {
    const createdNote = await NoteService.createNote(newNote.value)
    notes.value.push(createdNote)
    isCreateDialogOpen.value = false
    newNote.value = { title: '', content: null }
  } catch (error) {
    // Error handling is done in the service
  }
}

// Update a note
async function handleUpdateNote() {
  if (!selectedNote.value || selectedNote.value.id === null || selectedNote.value.id === undefined) return

  try {
    const updatedNote = await NoteService.updateNote(
      selectedNote.value.id, 
      selectedNote.value
    )
    
    // Update the note in the list
    const index = notes.value.findIndex(n => n.id === updatedNote.id)
    if (index !== -1) {
      notes.value[index] = updatedNote
    }
    
    isEditDialogOpen.value = false
  } catch (error) {
    // Error handling is done in the service
  }
}

// Delete a note
async function handleDeleteNote(id: number) {
  try {
    await NoteService.deleteNote(id)
    notes.value = notes.value.filter(note => note.id !== id)
  } catch (error) {
    // Error handling is done in the service
  }
}

// Open edit dialog
function openEditDialog(note: Note) {
  selectedNote.value = { ...note }
  isEditDialogOpen.value = true
}

// Helper function to convert payload to string
function payloadToString(payload: string | number): string {
  return typeof payload === 'number' ? payload.toString() : payload
}
</script>

<template>
  <div class="container mx-auto p-4">
    <!-- Create Note Dialog -->
    <Dialog v-model:open="isCreateDialogOpen">
      <DialogTrigger as-child>
        <Button variant="outline" class="mb-4">Create New Note</Button>
      </DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Create New Note</DialogTitle>
        </DialogHeader>
        <form @submit.prevent="handleCreateNote" class="space-y-4">
          <div>
            <Label>Title</Label>
            <Input 
              v-model="newNote.title" 
              placeholder="Note Title" 
              required 
            />
          </div>
          <div>
            <Label>Content</Label>
            <Textarea 
              :model-value="newNote.content ?? ''"
              @update:model-value="(payload) => newNote.content = payloadToString(payload)"
              placeholder="Note Content" 
            />
          </div>
          <Button type="submit">Create Note</Button>
        </form>
      </DialogContent>
    </Dialog>

    <!-- Notes Table -->
    <Table>
      <TableCaption>A list of your notes.</TableCaption>
      <TableHeader>
        <TableRow>
          <TableHead>ID</TableHead>
          <TableHead>Title</TableHead>
          <TableHead>Content</TableHead>
          <TableHead>Created At</TableHead>
          <TableHead>Actions</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        <TableRow v-for="note in notes" :key="note.id ?? note.title">
          <TableCell>{{ note.id }}</TableCell>
          <TableCell>{{ note.title }}</TableCell>
          <TableCell>{{ note.content }}</TableCell>
          <TableCell>{{ note.created_at }}</TableCell>
          <TableCell>
            <div class="flex space-x-2">
              <Button 
                size="sm" 
                variant="outline"
                @click="openEditDialog(note)"
              >
                Edit
              </Button>
              <Button 
                size="sm" 
                variant="destructive"
                @click="note.id !== null && note.id !== undefined ? handleDeleteNote(note.id) : undefined"
              >
                Delete
              </Button>
            </div>
          </TableCell>
        </TableRow>
      </TableBody>
    </Table>

    <!-- Edit Note Dialog -->
    <Dialog v-model:open="isEditDialogOpen">
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Edit Note</DialogTitle>
        </DialogHeader>
        <form @submit.prevent="handleUpdateNote" v-if="selectedNote" class="space-y-4">
          <div>
            <Label>Title</Label>
            <Input 
              v-model="selectedNote.title" 
              placeholder="Note Title" 
              required 
            />
          </div>
          <div>
            <Label>Content</Label>
            <Textarea 
              :model-value="selectedNote.content ?? ''"
              @update:model-value="(payload) => { if (selectedNote) selectedNote.content = payloadToString(payload) }"
              placeholder="Note Content" 
            />
          </div>
          <Button type="submit">Update Note</Button>
        </form>
      </DialogContent>
    </Dialog>
  </div>
</template>