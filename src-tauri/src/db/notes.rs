// src/db/notes.rs

use mysql_async::{prelude::*, Pool, Error as MySqlError};
use anyhow::Result;
use crate::models::Note;

pub struct NoteRepository {
    pool: Pool,
}

impl NoteRepository {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub async fn create_note(&self, note: &Note) -> Result<Note, MySqlError> {
        let mut conn = self.pool.get_conn().await?;

        let insert_query = r"INSERT INTO notes (title, content) VALUES (:title, :content)";
        let insert_params = params! {
            "title" => &note.title,
            "content" => &note.content
        };

        conn.exec_drop(insert_query, insert_params).await?;
        
        let last_id = conn.last_insert_id();

        let fetch_query = r"SELECT id, title, content, created_at, updated_at FROM notes WHERE id = :id";
        let fetch_params = params! {
            "id" => last_id
        };

        let created_note: Note = conn.exec_first(fetch_query, fetch_params)
            .await?
            .ok_or(MySqlError::Other("Note not found".into()))?;

        Ok(created_note)
    }

    pub async fn get_all_notes(&self) -> Result<Vec<Note>, MySqlError> {
        let mut conn = self.pool.get_conn().await?;

        let query = r"SELECT id, title, content, created_at, updated_at FROM notes ORDER BY created_at DESC";
        let notes: Vec<Note> = conn.exec(query, ()).await?;

        Ok(notes)
    }

    pub async fn get_note_by_id(&self, id: i64) -> Result<Note, MySqlError> {
        let mut conn = self.pool.get_conn().await?;

        let query = r"SELECT id, title, content, created_at, updated_at FROM notes WHERE id = :id";
        let params = params! {
            "id" => id
        };

        let note: Option<Note> = conn.exec_first(query, params).await?;

        note.ok_or(MySqlError::Other("Note not found".into()))
    }

    pub async fn update_note(&self, id: i64, note: &Note) -> Result<Note, MySqlError> {
        let mut conn = self.pool.get_conn().await?;

        let update_query = r"UPDATE notes SET title = :title, content = :content WHERE id = :id";
        let update_params = params! {
            "title" => &note.title,
            "content" => &note.content,
            "id" => id
        };

        conn.exec_drop(update_query, update_params).await?;

        // Fetch updated note
        self.get_note_by_id(id).await
    }

    pub async fn delete_note(&self, id: i64) -> Result<bool, MySqlError> {
        let mut conn = self.pool.get_conn().await?;
    
        let query = r"DELETE FROM notes WHERE id = :id";
        let params = params! {
            "id" => id
        };
    
        conn.exec_drop(query, params).await?;
    
        // Use a separate query to check affected rows
        let count_query = r"SELECT ROW_COUNT() as affected";
        let affected_rows: Option<i64> = conn.exec_first(count_query, ()).await?;
    
        Ok(affected_rows.map_or(false, |rows| rows > 0))
    }
}