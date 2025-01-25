// src/db.rs

pub mod notes;

use mysql_async::{Pool, Opts};
use mysql_async::prelude::Queryable;
use anyhow::Result;
use dotenv::dotenv;
use std::env;

pub async fn create_database_pool() -> Result<Pool> {
    // Load environment variables from .env
    dotenv().ok();

    // Extract the DATABASE_URL from .env or use a fallback (optional)
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
        eprintln!("DATABASE_URL is not set in .env. Falling back to the default URL.");
        "mysql://root:Ezeh4glamXgkaSeh@localhost:3307/app_db".to_string()
    });

    // Parse the database URL into connection options
    let opts = Opts::from_url(&database_url)?;
    let pool = Pool::new(opts);

    // Ensure the notes table exists by running the table creation query
    let mut conn = pool.get_conn().await?;
    conn.query_drop(crate::models::CREATE_NOTES_TABLE).await?;

    Ok(pool)
}
