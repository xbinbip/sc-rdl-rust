use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
// TODO Логгер который сохраняет логи в базу данных

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct LoggerRecord {
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub source: String,
    pub level: String
}

impl LoggerRecord {
    pub fn new(message: String, source: String, level: String) -> Self {
        Self {
            message,
            timestamp: Utc::now(),
            source,
            level,
        }
    }
}

macro_rules! log {
    () => {
        todo!();
    };
}

macro_rules! err {
    () => {
        todo!();
    };
}

macro_rules! warn {
    () => {
        todo!();
    };
}

macro_rules! info {
    ($msg:literal) => {
        todo!();
    };
}