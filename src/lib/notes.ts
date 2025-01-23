// src/lib/notes.ts

import { invoke } from '@tauri-apps/api/core'
import { useToast } from '@/components/ui/toast/use-toast'

// Define the Note interface to match your Rust backend
export interface Note {
    id?: number | null
    title: string
    content: string | null
    created_at?: string | null
    updated_at?: string | null
  }
export class NoteService {
  private static toast = useToast()

  // Type guard to check if the returned value is a Note
  private static isNote(obj: unknown): obj is Note {
    return obj !== null && typeof obj === 'object' && 
           'title' in obj && 
           (obj as Note).title !== undefined
  }

  // Create a new note
  static async createNote(note: Note): Promise<Note> {
    try {
      const createdNote = await invoke('create_note', { note }) as unknown
      
      if (!this.isNote(createdNote)) {
        throw new Error('Invalid note data returned')
      }

      this.toast.toast({
        title: 'Note Created',
        description: `Note "${note.title}" was successfully created.`,
      })
      return createdNote
    } catch (error) {
      this.toast.toast({
        title: 'Error',
        description: `Failed to create note: ${error}`,
        variant: 'destructive',
      })
      throw error
    }
  }

  // Get all notes
  static async getAllNotes(): Promise<Note[]> {
    try {
      const notes = await invoke('get_all_notes') as unknown
      
      if (!Array.isArray(notes) || !notes.every(this.isNote)) {
        throw new Error('Invalid notes data returned')
      }

      return notes
    } catch (error) {
      this.toast.toast({
        title: 'Error',
        description: `Failed to fetch notes: ${error}`,
        variant: 'destructive',
      })
      throw error
    }
  }

  // Get a note by ID
  static async getNoteById(id: number): Promise<Note> {
    try {
      const note = await invoke('get_note_by_id', { id }) as unknown
      
      if (!this.isNote(note)) {
        throw new Error('Invalid note data returned')
      }

      return note
    } catch (error) {
      this.toast.toast({
        title: 'Error',
        description: `Failed to fetch note with id ${id}: ${error}`,
        variant: 'destructive',
      })
      throw error
    }
  }

  // Update a note
  static async updateNote(id: number, note: Note): Promise<Note> {
    try {
      const updatedNote = await invoke('update_note', { id, note }) as unknown
      
      if (!this.isNote(updatedNote)) {
        throw new Error('Invalid updated note data returned')
      }

      this.toast.toast({
        title: 'Note Updated',
        description: `Note "${note.title}" was successfully updated.`,
      })
      return updatedNote
    } catch (error) {
      this.toast.toast({
        title: 'Error',
        description: `Failed to update note with id ${id}: ${error}`,
        variant: 'destructive',
      })
      throw error
    }
  }

  // Delete a note
  static async deleteNote(id: number): Promise<boolean> {
    try {
      const result = await invoke('delete_note', { id }) as unknown
      
      if (typeof result !== 'boolean') {
        throw new Error('Invalid delete result returned')
      }

      this.toast.toast({
        title: 'Note Deleted',
        description: `Note with id ${id} was successfully deleted.`,
      })
      return result
    } catch (error) {
      this.toast.toast({
        title: 'Error',
        description: `Failed to delete note with id ${id}: ${error}`,
        variant: 'destructive',
      })
      throw error
    }
  }
}