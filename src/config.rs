use config::{Config, File, ConfigError, Environment};     
use std::env;
use serde::{Deserialize};

#[derive(Debug, Clone, Deserialize)]
struct Spic {
    login: String,
    password: String,
    sessid: String
}

#[derive(Debug, Deserialize)]
enum DbType {
    Sqlite,
    Postgres
}

#[derive(Debug, Deserialize)]
struct Database {
    db_type: DbType,
    host: String,
    port: String,
    name: String
}

#[derive(Debug, Deserialize)]
struct Server {
    host: String,
    port: String
}

#[derive(Debug, Deserialize)]
struct Settings { 
    debug: bool,
    spic: Spic,
    server: Server,
    database: Database
}

impl Settings {
    fn new() -> Result<Self, ConfigError> {

        let args: Vec<String> = env::args().collect();
        let file_name: &str;

        if  args.contains(&"--debug".to_string()) {
            file_name = "./config/rdl-debug.toml";
        } else {
            file_name = ".config/rdl.toml";
        }

        let c = load_config()?;

        if let Ok(s) = c.try_deserialize::<Settings>() {
            Ok(s)
        } else {
            Err(ConfigError::Message(format!("Unable to parse config file: {}", file_name)))
        }
    }
}


fn load_config() -> Result<Config, ConfigError> {
    let s = config::Config::builder()
        .add_source(config::File::with_name("./src/config/rdl.toml"))
        .build();
    s
}
