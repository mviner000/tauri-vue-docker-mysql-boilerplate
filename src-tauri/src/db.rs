// src/db.rs

pub mod notes;

use mysql_async::{Pool, Opts, OptsBuilder};
use mysql_async::prelude::{Queryable, WithParams};
use anyhow::Result;

use dotenv::dotenv;
use std::env;

pub async fn create_database_pool() -> Result<Pool> {
    dotenv().ok(); // Load .env file
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    let opts = Opts::from_url(&database_url)?;
    let pool = Pool::new(opts);

    // Create notes table if it doesn't exist
    let mut conn = pool.get_conn().await?;
    conn.query_drop(crate::models::CREATE_NOTES_TABLE).await?;

    Ok(pool)
}