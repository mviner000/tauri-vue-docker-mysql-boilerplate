// src/models.rs

use serde::{Deserialize, Serialize};
use mysql_async::{Row, prelude::FromRow, FromRowError, prelude::FromValue};
use chrono::NaiveDateTime;
use mysql_async::Value;

pub const CREATE_NOTES_TABLE: &str = r#"
CREATE TABLE IF NOT EXISTS notes (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    title VARCHAR(255) NOT NULL,
    content TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
)"#;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: Option<i64>,
    pub title: String,
    pub content: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

impl Note {
    pub fn new(title: String, content: Option<String>) -> Self {
        Self {
            id: None,
            title,
            content,
            created_at: None,
            updated_at: None,
        }
    }
}

impl FromRow for Note {
    fn from_row(row: Row) -> Self {
        let title: String = row.get::<Option<String>, _>(1)
            .flatten()
            .unwrap_or_else(|| String::from("Untitled"));

        // Handle created_at
        let created_at = row.get::<Option<Value>, _>(3)
            .and_then(|opt_value| {
                opt_value.and_then(|value| match value {
                    Value::Date(year, month, day, hour, minute, second, _) => {
                        Some(format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", 
                            year, month, day, hour, minute, second))
                    },
                    Value::Bytes(bytes) => {
                        String::from_utf8_lossy(&bytes).to_string().into()
                    },
                    Value::Time(..) => None, // Use `..` to ignore all fields
                    _ => None
                })
            });

        // Handle updated_at
        let updated_at = row.get::<Option<Value>, _>(4)
            .and_then(|opt_value| {
                opt_value.and_then(|value| match value {
                    Value::Date(year, month, day, hour, minute, second, _) => {
                        Some(format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", 
                            year, month, day, hour, minute, second))
                    },
                    Value::Bytes(bytes) => {
                        String::from_utf8_lossy(&bytes).to_string().into()
                    },
                    Value::Time(..) => None, // Use `..` to ignore all fields
                    _ => None
                })
            });

        Note {
            id: row.get(0),
            title,
            content: row.get(2),
            created_at,
            updated_at,
        }
    }

    fn from_row_opt(row: Row) -> Result<Self, FromRowError> {
        Ok(Self::from_row(row))
    }
}

impl Note {
    pub fn created_at_datetime(&self) -> Option<NaiveDateTime> {
        self.created_at.as_ref()
            .and_then(|s| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").ok())
    }

    pub fn updated_at_datetime(&self) -> Option<NaiveDateTime> {
        self.updated_at.as_ref()
            .and_then(|s| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").ok())
    }
}