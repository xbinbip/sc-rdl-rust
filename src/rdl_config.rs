use config::{Config, File, ConfigError, Environment};   

use std::{env, sync::{Arc, RwLock}};
use serde::Deserialize;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref CONFIG: RwLock<Option<Settings>> = RwLock::new(None);
}



#[macro_export]
macro_rules! conf {
    ($name:ident) => {
        crate::rdl_config::get_config().as_ref().unwrap().$name.clone()
    };
}

#[derive(Debug, Deserialize, Clone)]
pub enum DebugLevel {
    #[serde(rename = "trace")]
    Trace,
    #[serde(rename = "info")]
    Info,
    #[serde(rename = "warn")]
    Warn,
    #[serde(rename = "debug")]
    Debug,
    #[serde(rename = "error")]
    Error,
    #[serde(rename = "off")]
    Off,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SpicConfig {
    pub login: String,
    pub password: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub db_type: String,
    pub sqlite: SqliteConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SqliteConfig {
    pub path: String,
    pub log_path: String,
    pub maximum_connection_pool_size: u32,
    pub minimum_connection_pool_size: u32,
    pub pool_acquire_timeout: u64,
    pub pool_max_lifetime: u64,
    pub pool_idle_timeout: u64
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub address: String,
    pub port: String,
    pub request_timeout: i32,
}


#[derive(Debug, Deserialize, Clone)]
pub struct Settings { 
    pub debug_level: DebugLevel,
    pub spic: SpicConfig,
    pub server: ServerConfig,
    pub database: DatabaseConfig
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {

        let args: Vec<String> = env::args().collect();
        let file_name: &str;

        if  args.contains(&"--debug".to_string()) {
            file_name = "./config/rdl-debug.toml";
        } else {
            file_name = "./config/rdl.toml";
        }

        let config_builder = Config::builder().
            add_source(File::with_name(file_name)).
            build()?;

        config_builder.try_deserialize::<Settings>().map_err(|e| {
            ConfigError::Message(format!(
                "Config parsing error: {}. Check if all required fields are present.",
                e
            ))
        })
        }
    }

pub fn init_config() -> Result<(), ConfigError> {
    let config = Settings::new()?;
    let mut config_lock = CONFIG.write().unwrap();
    *config_lock = Some(config);

    Ok(())
}

pub fn get_config() -> std::sync::RwLockReadGuard<'static, Option<Settings>> {
    CONFIG.read().unwrap()
}
