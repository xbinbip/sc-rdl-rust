mod config;
mod spic_client;

use serde::Deserialize;
use serde_json::json;

use tokio;

use crate::spic_client::init_client;

use chrono::{DateTime, Utc};


#[derive(Debug,Deserialize)]
struct AuthResponse {
    #[serde(rename = "IsAuthorized")]
    is_authorized: bool,
    #[serde(rename = "IsAuthenticated")]
    is_authenticated: bool,
    #[serde(rename = "UserId")]
    user_id: i32,
    #[serde(rename = "UserName")]
    user_name: String,
    #[serde(rename = "SessionId")]
    session_id: String,
    #[serde(rename = "ExpireDate")]
    expire_date: String
}

impl AuthResponse {
    fn from_json<'a>(json_data: &'a str) -> Self {
        serde_json::from_str(json_data).expect("Unable to parse auth response")
    }

    fn new() -> Self {
        AuthResponse {
            is_authorized: false,
            is_authenticated: false,
            user_id: 0,
            user_name: "".to_string(),
            session_id: "".to_string(),
            expire_date: "".to_string()
        }
    }
    
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // let config: Config = load_config().expect("Unable to load config file");
    // let datetime = Utc::now().to_rfc3339();



    let client = init_client();

    
    let response = client.authenticate().await;

    match response {
        Ok(response) => println!("Response: {:?}", serde_json::from_str::<AuthResponse>(&response.text().await.unwrap()).unwrap()),
        Err(error) => println!("Error: {}", error)
    }

    Ok(())
    // (let mut file = File::create("response.html").unwrap();
    // file.write_all(response.unwrap().text().unwrap().as_bytes()).unwrap();

}
