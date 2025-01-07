use serde::{Deserialize, Serialize, Deserializer};
use std::error::Error;
use chrono::{DateTime, TimeZone, Utc};

use keyring::Entry;
#[allow(unused, unused_variables, dead_code)]
use reqwest::{header, Client, Response};
use serde_json::json;

const DEFAULT_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:133.0) Gecko/20100101 Firefox/133.0";
const BASE_URL: &str = "http://login.scout-gps.ru/spic/";

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
fn deserialize_ms_date <'de, D>(date: D) -> Result<DateTime<Utc>, D::Error>
where D: Deserializer<'de>,
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

#[derive(Debug)]
pub struct SpicClient {
    client: Client,
    endpoints: SpicUrl,
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
}


impl SpicClient {
    
    fn new(client: Client) -> Self {
        SpicClient {
            client,
            endpoints: SpicUrl::default(),
            auth_token: None
        }
    }

    pub async fn authenticate(&mut self) -> Result<bool, SpicError> {

        if let Ok((token, expiration)) = get_stored_auth_data() {
            if expiration > Utc::now() {
                self.auth_token = Some(AuthToken::new(token, expiration));
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
            .post(&self.endpoints.spic_authorization_service.0)
            .json(&json_data)
            .send()
            .await?;

        match response.status() {
            reqwest::StatusCode::OK => {
                let body = response.text().await?;

                let auth_response = AuthResponse::from_json(&body)?;

                if auth_response.is_authorized && auth_response.is_authenticated {
                    let _ = store_auth_data(&auth_response.session_id, &auth_response.expire_date);
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
}

#[derive(Debug, thiserror::Error)]
pub enum SpicError {
    #[error("Authentication failed: {0}")]
    AuthenticationError(String),
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
    #[error("Invalid date format: {0}")]
    DateParseError(String)
}

#[derive(Debug)]
struct Url(String);

macro_rules! spic_url {
    ($expr:expr) => {
        Url(format!("{}/{}", BASE_URL, $expr))
    };
}

#[derive(Debug)]
struct SpicUrl {
    spic_authorization_service: Url,
    spic_units_service: Url,
    spic_unit_group_service: Url,
    spic_track_periods_mileage_statistics_service: Url,
    spic_track_periods_statistics_service: Url,
    spic_navigation_filtration_statistics_service: Url,
    spic_navigation_validation_statistics_service: Url,
    spic_motor_modes_statistics_service: Url,
    spic_statistics_controller_service: Url,
    spic_online_data_service: Url,
    spic_online_data_with_sensors_service: Url,
    spic_main_service: Url,
    spic_reports_service: Url,
    spic_fueling_defueling_statistics_service: Url,
    spic_discrete_sensors_statistics_service: Url,
    spic_fuel_flow_statistics_service: Url,
    spic_analog_sensor_statistics_service: Url,
    spic_tpm_event_service: Url,
    spic_fuel_event_service: Url,
}

impl Default for SpicUrl {
    fn default() -> Self {
        SpicUrl {
            spic_authorization_service: spic_url!("auth/rest/login"),
            spic_units_service: spic_url!("Units"),
            spic_unit_group_service: spic_url!("UnitGroups"),
            spic_track_periods_mileage_statistics_service: spic_url!(
                "TrackPeriodsMileageStatistics"
            ),
            spic_track_periods_statistics_service: spic_url!("TrackPeriodsStatistics"),
            spic_navigation_filtration_statistics_service: spic_url!(
                "NavigationFiltrationStatistics"
            ),
            spic_navigation_validation_statistics_service: spic_url!(
                "NavigationValidationStatistics"
            ),
            spic_motor_modes_statistics_service: spic_url!("MotorModesStatistics"),
            spic_statistics_controller_service: spic_url!("StatisticsController"),
            spic_online_data_service: spic_url!("OnlineData"),
            spic_online_data_with_sensors_service: spic_url!("OnlineDataWithSensors"),
            spic_main_service: spic_url!("Main"),
            spic_reports_service: spic_url!("Reports"),
            spic_fueling_defueling_statistics_service: spic_url!("FuelingDefuelingStatistics"),
            spic_discrete_sensors_statistics_service: spic_url!("DiscreteSensorsStatistics"),
            spic_fuel_flow_statistics_service: spic_url!("FuelFlowStatistics"),
            spic_analog_sensor_statistics_service: spic_url!("AnalogSensorStatistics"),
            spic_tpm_event_service: spic_url!("TpmEvent"),
            spic_fuel_event_service: spic_url!("FuelEvent"),
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
    session_id: Box<String>,
    #[serde(rename = "ExpireDate")]
    #[serde(deserialize_with = "deserialize_ms_date")]
    expire_date: DateTime<Utc>
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
            session_id: Box::new("".to_string()),
            expire_date: Utc::now()
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
    SpicClient::new(client)
}


