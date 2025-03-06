use core::time::Duration;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{FromRow, SqlitePool};
use crate::rdl_config::DatabaseConfig;
use crate::conf as conf;
use lazy_static::lazy_static;
use crate::spic_client::SpicUnit as SpicData;

lazy_static! {
    static ref DATABASE_CONFIG: DatabaseConfig = conf!(database);
}

macro_rules! execute_query {
    ($query:expr, $caller:literal, $pool:expr) => {
        $query.execute($pool).await.map_err(|e| DBError::SqliteError{
            caller: $caller,
            data: e.to_string(),
            source: e,
        })?;
    };
}

#[derive(Debug, FromRow)]
pub struct Database {
    pool: SqlitePool,
}

pub async fn create_sqlite_pool() -> Result<SqlitePool, DBError> {
    let database_url = &DATABASE_CONFIG.sqlite.log_path;
    SqlitePool::connect(&database_url).await.map_err(map_db_error("create_sqlite_pool", database_url.to_string()))
}

async fn set_pragma(pool: &SqlitePool, pragma: &str) -> Result<(), DBError> {
    execute_query!(
        sqlx::query(format!("PRAGMA {}", pragma).as_str()),
        "set_pragma",
        pool
    );
    Ok(())
}

impl Database {
    // pub async fn new() -> Result<Self, DBError> {
    //     let pool = create_sqlite_pool().await?;
    //     set_pragma(&pool, "PRAGMA journal_mode = WAL").await?;
    //     set_pragma(&pool, format!("PRAGMA busy_timeout = {}", DATABASE_CONFIG.sqlite.timeout).as_str()).await?;
    //     set_pragma(&pool, "PRAGMA foreign_keys = ON").await?;
    //     Ok(Self { pool })
    // }

    pub async fn new() -> Result<Self, DBError> {
        let pool = SqlitePoolOptions::new()
        .max_connections(DATABASE_CONFIG.sqlite.maximum_connection_pool_size)
        .min_connections(DATABASE_CONFIG.sqlite.minimum_connection_pool_size)
        .acquire_timeout(Duration::from_secs(DATABASE_CONFIG.sqlite.pool_acquire_timeout))
        .max_lifetime(Duration::from_secs(DATABASE_CONFIG.sqlite.pool_max_lifetime))
        .idle_timeout(Duration::from_secs(DATABASE_CONFIG.sqlite.pool_idle_timeout))
        .connect(&DATABASE_CONFIG.sqlite.path).await
        .map_err(|e| DBError::SqliteError{
            caller: "Database constuctor",
            data: format!("Database URL: {}", &DATABASE_CONFIG.sqlite.path),
            source: e,
        })?;

        set_pragma(&pool, "PRAGMA journal_mode = WAL").await?;

        dbg!(&pool);

        Ok(Self { pool })
    }



    async fn init_spic(&self) -> Result<(), DBError> {
        let query = sqlx::query(
            "CREATE TABLE IF NOT EXISTS spic_data (
                unit_id INTEGER PRIMARY KEY,
                brand TEXT,
                model TEXT,
                state_number TEXT,
                color TEXT,
                company_id INTEGER,
                description TEXT,
                garage_number TEXT,
                name TEXT NOT NULL,
                olson_id TEXT,
                owner TEXT,
                power INTEGER,
                registration TEXT,
                unit_type_id INTEGER,
                vin_number TEXT,
                year INTEGER,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )");

            execute_query!(query, "Database::init_spic", &self.pool);

            Ok(())
    }

    async fn init_logging(&self) -> Result<(), DBError> {
        let query = sqlx::query(
            "CREATE TABLE IF NOT EXISTS logging (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                level TEXT NOT NULL,
                message TEXT NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                source TEXT NOT NULL
            )");

        execute_query!(query, "Database::init_logging", &self.pool);

        Ok(())
    }

    pub async fn init(&self) -> Result<(), DBError> {
        self.init_spic().await?;
        self.init_logging().await?;

        Ok(())
    }

}

#[derive(thiserror::Error, Debug)]
pub enum DBError {
    #[error("Sqlite error: {source} \n in {caller}")]
    SqliteError{
        caller: &'static str,
        data: String,
        #[source]
        source: sqlx::Error,
    },
    
    #[error("Database error: {0}")]
    Error(#[from] std::io::Error)
}

fn map_db_error(caller: &'static str, data: String) -> impl FnOnce(sqlx::Error) -> DBError {
    move |e| DBError::SqliteError {
        caller,
        data,
        source: e,
    }
}

trait DatabaseOperations {

    async fn  insert(&self, pool: &SqlitePool) -> Result<(), DBError> ;

    async fn insert_or_update(&self, pool: &SqlitePool) -> Result<(), DBError>;

    async fn  delete(&self, pool: &SqlitePool) -> Result<(), DBError>;

    async fn  select(&self, pool: &SqlitePool) -> Result<(), DBError>;
}

impl DatabaseOperations for SpicData {
    async fn insert(&self, pool: &SqlitePool) -> Result<(), DBError> {
        let query = sqlx::query(
            "INSERT INTO spic_data (
                unit_id,
                brand,
                model,
                state_number,
                color,
                company_id,
                description,
                garage_number,
                name,
                olson_id,
                owner,
                power,
                registration,
                unit_type_id,
                vin_number,
                year
            ) VALUES (
                ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?
            )").bind(&self.id)
            .bind(&self.brand)
            .bind(&self.model)
            .bind(&self.state_number)
            .bind(&self.color)
            .bind(&self.company_id)
            .bind(&self.description)
            .bind(&self.garage_number)
            .bind(&self.name)
            .bind(&self.olson_id)
            .bind(&self.owner)
            .bind(&self.power)
            .bind(&self.registration)
            .bind(&self.type_id)
            .bind(&self.vin)
            .bind(&self.year);

            query.execute(pool).await.map_err(map_db_error("Insert on spicdata", format!("{:?}", self)))?;

            Ok(())
    }
    
    async fn insert_or_update(&self, pool: &SqlitePool) {
        todo!()
    }
    
    async fn  delete(&self, pool: &SqlitePool) {
        todo!()
    }
    
    async fn  select(&self, pool: &SqlitePool) {
        todo!()
    }
}

