#[allow(unused, unused_variables, dead_code)]

use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use std::{borrow::Cow, error::Error, io::empty, ops::Sub};

use keyring::Entry;

use reqwest::{header, Client};
use serde_json::json;

const DEFAULT_USER_AGENT: &'static str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:133.0) Gecko/20100101 Firefox/133.0";
const BASE_URL: &'static str = "http://login.scout-gps.ru/spic";

fn store_auth_data(token: &str, expiration: &DateTime<Utc>) -> Result<(), Box<dyn Error>> {
    let token_entry = Entry::new("sc-rdl", "auth_token")?;
    let expiration_entry = Entry::new("sc-rdl", "auth_expiration")?;

    // Convert DateTime to string format
    let expiration_str = expiration.to_rfc3339();

    token_entry.set_password(token)?;
    expiration_entry.set_password(&expiration_str)?;
    Ok(())
}

fn get_stored_auth_data() -> Result<(String, DateTime<Utc>), Box<dyn Error>> {
    let token_entry = Entry::new("sc-rdl", "auth_token")?;
    let expiration_entry = Entry::new("sc-rdl", "auth_expiration")?;

    let token = token_entry.get_password()?;
    let expiration_str = expiration_entry.get_password()?;

    // Parse string back to DateTime
    let expiration = DateTime::parse_from_rfc3339(&expiration_str)?.with_timezone(&Utc);

    Ok((token, expiration))
}
fn deserialize_ms_date<'de, D>(date: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let date_str: String = String::deserialize(date)?;

    let ms = date_str
        .trim_start_matches("/Date(")
        .trim_end_matches(")/")
        .split(|c| c == '+' || c == '-')
        .next()
        .unwrap()
        .parse::<i64>()
        .unwrap();

    Ok(Utc.timestamp_millis_opt(ms).unwrap())
}

// TODO: define  a custom json parser for response from server

macro_rules! endpoint {
    ($ep:ident) => {
        SpicEndpoint::$ep.0
    };
}

#[derive(Debug)]
pub struct SpicClient {
    client: Client,
    auth_token: Option<AuthToken>,
}

#[derive(Debug)]
struct AuthToken {
    token: String,
    expiration: DateTime<Utc>,
}

impl AuthToken {
    fn new(token: String, date: DateTime<Utc>) -> Self {
        AuthToken {
            token,
            expiration: date,
        }
    }

    fn is_expired(&self) -> bool {
        self.expiration < Utc::now()
    }

    fn is_valid(&self) -> bool {
        // TODO: Check for validity of token, try to use some API request
        // to check if token is valid

        true
    }
}

impl SpicClient {
    fn new(client: Client) -> Self {
        SpicClient {
            client,
            auth_token: None,
        }
    }

    fn authenticated_client(&self, token: &str) -> Client {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            "ScoutAuthorization",
            header::HeaderValue::from_str(token).unwrap(),
            );
            
        headers.insert(
            "Content-Type",
            header::HeaderValue::from_static("application/json"),
            
        );

        headers.insert(
            "Accept",
            header::HeaderValue::from_static("application/json; charset=utf-8"),
        );
        Client::builder()
            .user_agent(DEFAULT_USER_AGENT)
            .default_headers(headers)
            .build()
            .expect("Unable to create authenticated reqwest client")
    }

    pub async fn authenticate(&mut self) -> Result<bool, SpicError> {

        if let Ok((token, expiration)) = get_stored_auth_data() {
            println!(
                "Stored auth data found, \n Token: {}\n Expiration: {}",
                token, expiration
            );
            self.auth_token = Some(AuthToken::new(token, expiration));

            if !self.auth_token.as_ref().unwrap().is_expired() {
                println!("Token is not expired");
                self.client = self.authenticated_client(&self.auth_token.as_ref().unwrap().token);
                return Ok(true);
            }
        }

        // TODO: get credentials from config
        let json_data = json!({
            "Login": "kgm@redlineekb.ru",
            "Password": "5Amxqv",
            "TimeZoneOlsonId": "Asia/Yekaterinburg",
            "CultureName": "ru-ru",
            "UiCultureName": "ru-ru"
        });

        let response = self
            .client
            .post(endpoint!(AUTHORIZATION_SERVICE))
            .json(&json_data)
            .send()
            .await?;

        match response.status() {
            reqwest::StatusCode::OK => {
                let body = response.text().await?;

                println!("Requesting authentication token...");

                let auth_response = AuthResponse::from_json(&body)?;

                if auth_response.is_authorized && auth_response.is_authenticated {
                    let _ = store_auth_data(&auth_response.session_id, &auth_response.expire_date);

                    println!(
                        "Authentication successful, user id: {}, session id: {}",
                        auth_response.user_id, auth_response.session_id
                    );
                    
                    self.client = self.authenticated_client(&auth_response.session_id);

                    Ok(true)
                } else {
                    Err(SpicError::AuthenticationError(
                        "Authentication failed".to_string(),
                    ))
                }
            }
            _ => {
                // TODO: handle error and add logging instead
                println!(
                    "Authentication failed with status code: {}",
                    response.status()
                );
                return Ok(false);
            }
        }
    }

    pub async fn number_of_units(&self) -> Result<i32, SpicError> {

        let response = self
            .client
            .get(endpoint!(UNITS_NUMBER_SERVICE))
            .send()
            .await?;


        match response.status() {
            // TODO: rewrite error handling using spicerror
            reqwest::StatusCode::OK => {
                let body = response.text().await?;
                let number_of_units = body.parse::<i32>()?;
                Ok(number_of_units)
            }
            _ => {
                // TODO: handle error and add logging instead
                println!(
                    "failed to get number of units with status code: {}",
                    response.status()
                );
                return Ok(0);
            }
        }
    }

    pub async fn unit_list(&self) -> Result<Vec<SpicUnit>, SpicError> {

        let response = self
            .client
            .get(endpoint!(UNIT_LIST_SERVICE))
            .send()
            .await?;

        match response.status() {
            reqwest::StatusCode::OK => {
                let body = response.text().await?;
                let unit_list = SpicUnitList::from_json(&body)?;
                Ok(unit_list.units)
            }
            _ => {
                // TODO: handle error and add logging instead
                println!(
                    "failed to get unit list with status code: {}",
                    response.status()
                );
                return Ok(vec![]);
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SpicError {

    #[error("Network error: {0}/")]
    NetworkError(#[from] reqwest::Error),

    #[error("Invalid date format: {0}")]
    DateParseError(String),

    #[error("JSON parsing error: {source} \n in {caller}")]
    JsonError{
        caller: &'static str,
        #[source]
        source: serde_json::Error
    },

    #[error("Parse int error: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),

    #[error("Authentication error: {0}")]
    AuthenticationError(String),
}

#[derive(Debug)]
struct Url(&'static str);

impl Url {
    fn new(url: &'static str) -> Self {
        Url(url)
    }
}

macro_rules! spic_url {
    ($path:expr) => {
        Url(concat!("http://login.scout-gps.ru/spic", $path))
    };
}

#[derive(Debug)]
struct SpicEndpoint;

#[allow(unused)]
impl SpicEndpoint {
    const AUTHORIZATION_SERVICE: Url = spic_url!("/auth/rest/login");
    const AUTHORIZATION_LOGOUT: Url = spic_url!("/auth/rest/logout");
    const UNITS_NUMBER_SERVICE: Url = spic_url!("/Units/rest/");
    const UNIT_LIST_SERVICE: Url = spic_url!("/Units/rest/GetAllUnits");
    const UNIT_GROUP_SERVICE: Url = spic_url!("/UnitGroups");
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SpicUnit {
    #[serde(rename = "Brand")]
    brand: String,
    #[serde(rename = "Color")]
    color: String,
    #[serde(rename = "CompanyId")]
    company_id: i32,
    #[serde(rename = "Description")]
    description: String,
    #[serde(rename = "GarageNumber")]
    garage_number: String,
    #[serde(rename = "Model")]
    model: String,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "OlsonId")]
    olson_id: String,
    #[serde(rename = "Owner")]
    owner: String,
    #[serde(rename = "Power")]
    power: String,
    #[serde(rename = "Registration")]
    registration: String,
    #[serde(rename = "StateNumber")]
    state_number: String,
    #[serde(rename = "UnitId")]
    id: i32,
    #[serde(rename = "UnitTypeId")]
    #[serde(default)]
    type_id: Option<i32>,
    #[serde(rename = "VinNumber")]
    vin: String,
    #[serde(rename = "Year")]
    year: String,
}


impl SpicUnit {
    fn from_json<'a>(json_data: &'a String) -> Result<SpicUnit, SpicError> {
        match serde_json::from_str::<SpicUnit>(&json_data) {
            Ok(data) => Ok(data),
            Err(e) => {
                Err(SpicError::JsonError { caller: "SpicUnit::from_json", source: e })
            }
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct SpicUnitList {
    #[serde(rename = "Units")]
    units: Vec<SpicUnit>,
}

impl SpicUnitList {
    fn from_json<'a>(json_data: &'a String) -> Result<SpicUnitList, SpicError> {
        match serde_json::from_str(&json_data) {
            Ok(data) => Ok(data),
            Err(e) => {
                println!("{}", e);
                Err(SpicError::JsonError{ caller: "SpicUnitList::from_json", source: e })
            }
        }
    }
    
}

#[derive(Debug)]
struct SubscriptionHandler {
    subscriptions: std::collections::HashMap<i32, Subscription>,
}

impl SubscriptionHandler {
    fn new() -> Self {
        SubscriptionHandler {
            subscriptions: std::collections::HashMap::with_capacity(5),
        }
    }

    fn add_subscription(&mut self, unit_id: i32, uuid: String) {
        self.subscriptions.insert(unit_id, Subscription::new(uuid));
    }

    fn remove_subscription(&mut self, unit_id: i32) {
        self.subscriptions.remove(&unit_id);
    }
    fn is_exist(&self, unit_id: i32) -> bool {
        self.subscriptions.contains_key(&unit_id) 
    }

    fn clear_expired(&mut self) {
        self.subscriptions.retain(|_, subscription| !subscription.is_expired());
    }
}

#[derive(Debug)]
struct Subscription {
    uuid: String,
    created_at: DateTime<Utc>,
}

impl Subscription {
    fn new(uuid: String) -> Self {
        Self {
            uuid,
            created_at: Utc::now(),
        }
    }
    fn is_expired(&self) -> bool {
        let now = Utc::now();
        let diff = now - self.created_at;
        diff.num_minutes() > 10
    }
}

enum OnlineDataErrorCodes {
    BadRequest = 200,
    RightsViolation = 201,
    InternalError = 202,
    TerminalNotFound = 203,
    SubscriptionNotFound = 204,
    OnlineDataNotFound = 205,
}

enum OnlineDataStatus {
    None,
    Ok,
    Error,
    Busy,
    PartialOk,
}

#[derive(Debug, Deserialize, Serialize)]
struct OnlineData{
    address: String,
    connection_date_time: DateTime<Utc>,
    device_id: DeviceId,
    is_navigation_valid: bool,
    last_message_time: DateTime<Utc>,
    navigation: NavigationData,
    navigation_time: DateTime<Utc>,
    total_messages: i32,
}
#[derive(Debug, Deserialize, Serialize)]
struct DeviceId {
    protocol: String,
    serial_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct NavigationData {
    altitude_meters: i32,
    angle: i32,
    hardware_validation: Option<String>,
    location: Location,
    navigation_system_type: String,
    satellites_count: i8,
    speed: i32,
}
#[derive(Debug, Deserialize, Serialize)]
struct Location {
    latitude: f64,
    longitude: f64,
}

impl OnlineData {
    fn from_json<'a>(json_data: &'a String) -> Result<OnlineData, SpicError> {
        match serde_json::from_str::<OnlineData>(&json_data) {
            Ok(data) => Ok(data),
            Err(e) => {
                Err(SpicError::JsonError { caller: "OnlineData::from_json", source: e })
            }
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AuthResponse {
    #[serde(rename = "IsAuthorized")]
    is_authorized: bool,
    #[serde(rename = "IsAuthenticated")]
    is_authenticated: bool,
    #[serde(rename = "UserId")]
    user_id: i32,
    #[serde(rename = "UserName")]
    user_name: String,
    #[serde(rename = "SessionId")]
    session_id: Cow<'static, String>,
    #[serde(rename = "ExpireDate")]
    #[serde(deserialize_with = "deserialize_ms_date")]
    expire_date: DateTime<Utc>,
}

impl AuthResponse {
    fn from_json<'a>(json_data: &'a String) -> Result<AuthResponse, SpicError> {
        if let Ok(data) = serde_json::from_str::<AuthResponse>(&json_data) {
            Ok(data)
        } else {
            return Err(SpicError::AuthenticationError(
                "Unable to parse authentication response".to_string(),
            ));
        }
    }
    fn new() -> Self {
        AuthResponse {
            is_authorized: false,
            is_authenticated: false,
            user_id: 0,
            user_name: "".to_string(),
            session_id: Cow::Owned("".to_string()),
            expire_date: Utc::now(),
        }
    }
}

pub fn init_client() -> SpicClient {
    let mut headers = header::HeaderMap::new();
    headers.insert(
        "Content-Type",
        header::HeaderValue::from_static("application/json"),
    );
    headers.insert(
        "Accept",
        header::HeaderValue::from_static("application/json; charset=utf-8"),
    );

    let client = Client::builder()
        .user_agent(DEFAULT_USER_AGENT)
        .default_headers(headers)
        .build()
        .expect("Unable to create reqwest client");

    dbg!(&client);

    let sh = SubscriptionHandler::new();

    SpicClient::new(client)
}

fn is_normal<T: Send + Sync + Unpin + Sized>() {}

/// implement the necessary traits (`Send`, `Sync`, `Unpin`, and `Sized`)
/// such that they can be used in async contexts.
#[test]
fn test_is_normal() {
    is_normal::<SpicClient>();
    is_normal::<AuthToken>();
    is_normal::<SpicEndpoint>();
}

#[cfg(test)]
mod subscription_tests {
    use super::*;
    use chrono::{Utc, Duration};

    #[test]
    fn test_add_subscription() {
        let mut handler = SubscriptionHandler::new();
        handler.add_subscription(1, "uuid-1".to_string());

        assert!(handler.is_exist(1));
    }

    #[test]
    fn test_add_subscription_duplicate() {
        let mut handler = SubscriptionHandler::new();
        handler.add_subscription(1, "uuid-1".to_string());
        handler.add_subscription(1, "uuid-2".to_string());

        // Ensure that the latest uuid is stored
        assert_eq!(handler.subscriptions.get(&1).unwrap().uuid, "uuid-2");
    }

    #[test]
    fn test_remove_subscription() {
        let mut handler = SubscriptionHandler::new();
        handler.add_subscription(1, "uuid-1".to_string());
        handler.remove_subscription(1);

        assert!(!handler.is_exist(1));
    }

    #[test]
    fn test_remove_nonexistent_subscription() {
        let mut handler = SubscriptionHandler::new();
        handler.remove_subscription(1); // Removing without adding

        assert!(!handler.is_exist(1));
    }

    #[test]
    fn test_is_exist() {
        let mut handler = SubscriptionHandler::new();
        handler.add_subscription(1, "uuid-1".to_string());

        assert!(handler.is_exist(1));
        assert!(!handler.is_exist(2)); // Nonexistent
    }

    #[test]
    fn test_clear_expired() {
        let mut handler = SubscriptionHandler::new();
        handler.add_subscription(1, "uuid-1".to_string());
        
        // Simulate expiration
        let expired_subscription = Subscription {
            uuid: "uuid-expired".to_string(),
            created_at: Utc::now() - Duration::days(1), // Assume expired
        };
        handler.subscriptions.insert(2, expired_subscription);

        assert!(handler.subscriptions.get(&2).unwrap().is_expired());
        println!("Subscription 2 is expired: {}", handler.subscriptions.get(&2).unwrap().is_expired());

        handler.clear_expired();

        assert!(handler.is_exist(1)); // Should still exist
        assert!(!handler.is_exist(2)); // Should be removed
    }

    #[test]
    fn test_clear_expired_no_expired() {
        let mut handler = SubscriptionHandler::new();
        handler.add_subscription(1, "uuid-1".to_string());
        handler.add_subscription(2, "uuid-2".to_string());

        // No subscriptions are expired
        handler.clear_expired();

        assert!(handler.is_exist(1));
        assert!(handler.is_exist(2));
    }
}