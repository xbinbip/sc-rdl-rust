use chrono::Duration;
#[allow(unused, unused_variables, dead_code)]
// Работает? не трогай. Ретрай обязательно сделать, с проверкой того что вернули подписки, т.к у них есть задержка
use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use std::{
    borrow::Cow,
    collections::HashMap,
    error::Error,
    future::Future,
    sync::{Arc, Mutex},
};

use keyring::Entry;

use reqwest::{header, Client};
use serde_json::json;

use crate::rdl_config::{SpicConfig, CONFIG};
use crate::conf as conf;

const DEFAULT_USER_AGENT: &'static str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:133.0) Gecko/20100101 Firefox/133.0";
const BASE_URL: &'static str = "http://login.scout-gps.ru/spic";
const LOCAL_TIME_SHIFT: i64 = 5;

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
        .expect("Failed to parse date");

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
    subman: Arc<Mutex<SubscriptionManager>>,
    config: Option<SpicConfig>,
}

// Add retry mechanism for network requests
async fn with_retry<F, T, E>(f: F, max_retries: u32) -> Result<T, E>
where
    F: Fn() -> dyn Future<Output = Result<T, E>>,
{
    //TODO: Implement exponential backoff retry logic
    todo!();
}

#[derive(Debug, Deserialize, Serialize)]
struct Uuid {
    #[serde(rename = "Id")]
    uuid: String,
}

impl Uuid {
    fn from_subscription_response(json_data: &String) -> Result<Uuid, SpicError> {
        match serde_json::from_str::<Uuid>(&json_data) {
            Ok(data) => Ok(data),
            Err(e) => Err(SpicError::JsonError {
                caller: "Uuid::from_subscription_response",
                source: e,
                data: json_data.to_string(),
            }),
        }
    }

    fn from_string(uuid: String) -> Self {
        Uuid { uuid }
    }

    fn as_str(&self) -> &str {
        &self.uuid
    }

    fn as_string(&self) -> String {
        self.uuid.clone()
    }
}

#[derive(Debug)]
struct AuthToken {
    token: Uuid,
    expiration: DateTime<Utc>,
}

impl AuthToken {
    fn new(token: String, date: DateTime<Utc>) -> Self {
        AuthToken {
            token: Uuid::from_string(token),
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

impl SpicClient{
    fn new(client: Client) -> Self {

        let config:SpicConfig = conf!(spic);

        SpicClient {
            client,
            auth_token: None,
            subman: Arc::new(Mutex::new(SubscriptionManager::new())),
            config: Some(config),
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

        let _config = self.config.as_ref().unwrap();

        let (login, password) = (_config.login.as_str(), _config.password.as_str());


        if let Ok((token, expiration)) = get_stored_auth_data() {
            println!(
                "Stored auth data found, \n Token: {}\n Expiration: {}",
                token, expiration
            );
            self.auth_token = Some(AuthToken::new(token, expiration));

            if !self.auth_token.as_ref().unwrap().is_expired() {
                println!("Token is not expired");
                self.client =
                    self.authenticated_client(&self.auth_token.as_ref().unwrap().token.as_string());
                return Ok(true);
            }
        }


        // TODO: get credentials from config
        let json_data = json!({
            "Login": login,
            "Password": password,
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
                    let _ = store_auth_data(&auth_response.session_id,
                         &{auth_response.expire_date - Duration::hours(LOCAL_TIME_SHIFT)});

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
        let response = self.client.get(endpoint!(UNIT_LIST_SERVICE)).send().await?;

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

    pub async fn get_online_data(&self, unit_id: i32) -> Result<OnlineData, SpicError> {
        let _subman = self.subman.clone();

        let json_request = json!({
            "UnitIds" : [unit_id]
        });

        let req = self
            .client
            .post(endpoint!(ONLINE_DATA_SUBSCRIBE))
            .json(&json_request);

        let response = req.send().await?;

        match response.status() {
            reqwest::StatusCode::OK => {
                let body = response.text().await?;
                let data = serde_json::from_str::<SubscriptionResponse>(&body);

                match data {
                    Ok(data) => {
                        if data.state.is_ok() {
                            let subscription_token = data.session_id;
                            _subman
                                .lock()
                                .unwrap()
                                .add_subscription(unit_id, subscription_token.as_string());
                        } else {
                            println!("failed to subscribe to unit with id: {}", unit_id);
                            return Err(SpicError::SubscriptionError {
                                caller: "SpicClient::get_online_data",
                                data: format!("failed to subscribe to unit with id: {}", unit_id),
                                source: std::io::Error::new(
                                    std::io::ErrorKind::Other,
                                    "failed to subscribe to unit",
                                ),
                            });
                        }
                    }
                    Err(e) => {
                        return Err(SpicError::JsonError {
                            caller: "SpicClient::get_online_data",
                            source: e,
                            data: body.to_string(),
                        })
                    }
                }
            }
            _ => {
                return Err(SpicError::NetworkError(
                    response.error_for_status().unwrap_err(),
                ));
            }
        }

        let subscribed_json = json!({
            "Id": _subman.lock().unwrap().subscriptions.get(&unit_id).unwrap().uuid,
        });

        let response = self
            .client
            .post(endpoint!(ONLINE_DATA_GET))
            .json(&subscribed_json)
            .send()
            .await?;

        match response.status() {
            reqwest::StatusCode::OK => {
                let body = response.text().await?;
                let online_data_col = serde_json::from_str::<ODResponse>(&body);

                match online_data_col {
                    Ok(data) => {
                        if data.is_ok() {
                            let online_data = data
                                .online_data_collection
                                .data_collection
                                .unwrap()
                                .first()
                                .unwrap()
                                .clone();
                            return Ok(online_data);
                        } else {
                            return Err(SpicError::ResponseError {
                                caller: "SpicClient::get_online_data",
                                data: format!(
                                    "failed to get online data for unit with id: {}",
                                    unit_id
                                ),
                                source: std::io::Error::new(
                                    std::io::ErrorKind::Other,
                                    "failed to get online data",
                                ),
                            });
                        }
                    }
                    Err(e) => {
                        return Err(SpicError::JsonError {
                            caller: "SpicClient::get_online_data",
                            source: e,
                            data: body.to_string(),
                        })
                    }
                }
            }
            _ => {
                return Err(SpicError::NetworkError(
                    response.error_for_status().unwrap_err(),
                ));
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
    JsonError {
        caller: &'static str,
        data: String,
        #[source]
        source: serde_json::Error,
    },

    #[error("Parse int error: {0}")]
    ParseIntError(#[from] std::num::ParseIntError),

    #[error("Authentication error: {0}")]
    AuthenticationError(String),

    #[error("Subscription error: {source} \n in {caller}")]
    SubscriptionError {
        caller: &'static str,
        data: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Response error: {data} \n in {caller}")]
    ResponseError {
        caller: &'static str,
        data: String,
        #[source]
        source: std::io::Error,
    },
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
    const ONLINE_DATA_SUBSCRIBE: Url = spic_url!("/OnlineDataService/rest/Subscribe");
    const ONLINE_DATA_GET: Url = spic_url!("/OnlineDataService/rest/GetOnlineData");
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SpicUnit {
    #[serde(rename = "Brand")]
    pub brand: String,
    #[serde(rename = "Color")]
    pub color: String,
    #[serde(rename = "CompanyId")]
    pub company_id: i32,
    #[serde(rename = "Description")]
    pub description: String,
    #[serde(rename = "GarageNumber")]
    pub garage_number: String,
    #[serde(rename = "Model")]
    pub model: String,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "OlsonId")]
    pub olson_id: String,
    #[serde(rename = "Owner")]
    pub owner: String,
    #[serde(rename = "Power")]
    pub power: String,
    #[serde(rename = "Registration")]
    pub registration: String,
    #[serde(rename = "StateNumber")]
    pub state_number: String,
    #[serde(rename = "UnitId")]
    pub id: i32,
    #[serde(rename = "UnitTypeId")]
    #[serde(default)]
    pub type_id: Option<i32>,
    #[serde(rename = "VinNumber")]
    pub vin: String,
    #[serde(rename = "Year")]
    pub year: String,
}

impl SpicUnit {
    fn from_json<'a>(json_data: &'a String) -> Result<SpicUnit, SpicError> {
        match serde_json::from_str::<SpicUnit>(&json_data) {
            Ok(data) => Ok(data),
            Err(e) => Err(SpicError::JsonError {
                caller: "SpicUnit::from_json",
                source: e,
                data: json_data.to_string(),
            }),
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
                Err(SpicError::JsonError {
                    caller: "SpicUnitList::from_json",
                    source: e,
                    data: json_data.to_string(),
                })
            }
        }
    }
}

#[derive(Debug)]
struct SubscriptionManager {
    subscriptions: std::collections::HashMap<i32, Subscription>,
}

impl SubscriptionManager {
    fn new() -> Self {
        SubscriptionManager {
            subscriptions: std::collections::HashMap::with_capacity(5),
        }
    }

    fn add_subscription(&mut self, unit_id: i32, uuid: String) {
        println!("add subscription for unit_id: {}", unit_id);
        println!("Subscription id: {}", uuid);
        self.subscriptions.insert(unit_id, Subscription::new(uuid));
    }

    fn remove_subscription(&mut self, unit_id: i32) {
        self.subscriptions.remove(&unit_id);
    }
    fn is_exist(&self, unit_id: i32) -> bool {
        self.subscriptions.contains_key(&unit_id)
    }

    fn clear_expired(&mut self) {
        self.subscriptions
            .retain(|_, subscription| !subscription.is_expired());
    }

    fn get_subscription(&self, unit_id: i32) -> Option<&Subscription> {
        if self.is_exist(unit_id) && !self.subscriptions.get(&unit_id).unwrap().is_expired() {
            self.subscriptions.get(&unit_id)
        } else {
            None
        }
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

enum ODErrorCodes {
    BadRequest = 200,
    RightsViolation = 201,
    InternalError = 202,
    TerminalNotFound = 203,
    SubscriptionNotFound = 204,
    OnlineDataNotFound = 205,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
enum ODStatus {
    #[serde(rename = "Ok")]
    Ok,
    #[serde(rename = "Error")]
    Error,
    #[serde(rename = "Busy")]
    Busy,
    #[serde(rename = "PartialOk")]
    PartialOk,
    #[serde(rename = "None")]
    None,
}

#[derive(Debug, Deserialize, Serialize)]
struct ODResponse {
    #[serde(rename = "OnlineDataCollection")]
    online_data_collection: ODCollection,
    #[serde(rename = "State")]
    state: ODResponseState,
}

impl ODResponse {
    fn from_json<'a>(json_data: &'a String) -> Result<ODResponse, SpicError> {
        match serde_json::from_str(&json_data) {
            Ok(data) => Ok(data),
            Err(e) => Err(SpicError::JsonError {
                caller: "OnlineDataResponse::from_json",
                source: e,
                data: json_data.to_string(),
            }),
        }
    }
    fn is_ok(&self) -> bool {
        self.state.is_ok()
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct ODCollection {
    #[serde(rename = "DataCollection")]
    data_collection: Option<Vec<OnlineData>>,
    #[serde(rename = "Targets")]
    targets: Vec<i32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SubscriptionResponse {
    #[serde(rename = "SessionId")]
    session_id: Uuid,
    #[serde(rename = "State")]
    state: ODResponseState,
}

#[derive(Debug, Deserialize, Serialize)]
struct ODValue {
    #[serde(rename = "Value")]
    value: ODStatus,
}

#[derive(Debug, Deserialize, Serialize)]
struct ODResponseState {
    #[serde(rename = "ErrorCodes")]
    error_codes: Vec<i32>,
    #[serde(rename = "Status")]
    status: ODValue,
}

impl ODResponseState {
    fn is_ok(&self) -> bool {
        self.status.value == ODStatus::Ok
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OnlineData {
    #[serde(rename = "Address")]
    address: String,
    #[serde(
        rename = "ConnectionDateTime",
        deserialize_with = "deserialize_ms_date"
    )]
    connection_date_time: DateTime<Utc>,
    #[serde(rename = "DeviceId")]
    device_id: DeviceId,
    #[serde(rename = "IsNavigationValid")]
    is_navigation_valid: bool,
    #[serde(rename = "LastMessageTime", deserialize_with = "deserialize_ms_date")]
    last_message_time: DateTime<Utc>,
    #[serde(rename = "Navigation")]
    navigation: NavigationData,
    #[serde(rename = "NavigationTime", deserialize_with = "deserialize_ms_date")]
    navigation_time: DateTime<Utc>,
    #[serde(rename = "TotalMessages")]
    total_messages: i32,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
struct DeviceId {
    #[serde(rename = "Protocol")]
    protocol: HashMap<String, String>,
    #[serde(rename = "SerialId")]
    serial_id: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "PascalCase")]
struct NavigationData {
    altitude_meters: i32,
    angle: i32,
    hardware_validation: Option<String>,
    location: Location,
    navigation_system_type: String,
    satellites_count: i8,
    speed: i32,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "PascalCase")]
struct Location {
    latitude: f64,
    longitude: f64,
}

impl OnlineData {
    fn from_json<'a>(json_data: &'a String) -> Result<OnlineData, SpicError> {
        match serde_json::from_str::<OnlineData>(&json_data) {
            Ok(data) => Ok(data),
            Err(e) => Err(SpicError::JsonError {
                caller: "OnlineData::from_json",
                source: e,
                data: json_data.to_string(),
            }),
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
    use chrono::{Duration, Utc};

    #[test]
    fn test_add_subscription() {
        let mut handler = SubscriptionManager::new();
        handler.add_subscription(1, "uuid-1".to_string());

        assert!(handler.is_exist(1));
    }

    #[test]
    fn test_add_subscription_duplicate() {
        let mut handler = SubscriptionManager::new();
        handler.add_subscription(1, "uuid-1".to_string());
        handler.add_subscription(1, "uuid-2".to_string());

        // Ensure that the latest uuid is stored
        assert_eq!(handler.subscriptions.get(&1).unwrap().uuid, "uuid-2");
    }

    #[test]
    fn test_remove_subscription() {
        let mut handler = SubscriptionManager::new();
        handler.add_subscription(1, "uuid-1".to_string());
        handler.remove_subscription(1);

        assert!(!handler.is_exist(1));
    }

    #[test]
    fn test_remove_nonexistent_subscription() {
        let mut handler = SubscriptionManager::new();
        handler.remove_subscription(1); // Removing without adding

        assert!(!handler.is_exist(1));
    }

    #[test]
    fn test_is_exist() {
        let mut handler = SubscriptionManager::new();
        handler.add_subscription(1, "uuid-1".to_string());

        assert!(handler.is_exist(1));
        assert!(!handler.is_exist(2)); // Nonexistent
    }

    #[test]
    fn test_clear_expired() {
        let mut handler = SubscriptionManager::new();
        handler.add_subscription(1, "uuid-1".to_string());

        // Simulate expiration
        let expired_subscription = Subscription {
            uuid: "uuid-expired".to_string(),
            created_at: Utc::now() - Duration::days(1), // Assume expired
        };
        handler.subscriptions.insert(2, expired_subscription);

        assert!(handler.subscriptions.get(&2).unwrap().is_expired());
        println!(
            "Subscription 2 is expired: {}",
            handler.subscriptions.get(&2).unwrap().is_expired()
        );

        handler.clear_expired();

        assert!(handler.is_exist(1)); // Should still exist
        assert!(!handler.is_exist(2)); // Should be removed
    }

    #[test]
    fn test_clear_expired_no_expired() {
        let mut handler = SubscriptionManager::new();
        handler.add_subscription(1, "uuid-1".to_string());
        handler.add_subscription(2, "uuid-2".to_string());

        // No subscriptions are expired
        handler.clear_expired();

        assert!(handler.is_exist(1));
        assert!(handler.is_exist(2));
    }

    #[test]
    fn test_get_subscription() {
        let mut handler = SubscriptionManager::new();
        handler.add_subscription(1, "uuid-1".to_string());
        handler.add_subscription(2, "uuid-2".to_string());

        assert!(handler.get_subscription(1).is_some());
        assert!(handler.get_subscription(2).is_some());
        assert!(handler.get_subscription(3).is_none());
    }
}
