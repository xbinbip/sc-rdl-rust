use std::error::Error;
use serde::{Deserialize, Serialize};

#[allow(unused,unused_variables,dead_code)]

use reqwest::{header, Client, Response};
use serde_json::json;

const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:133.0) Gecko/20100101 Firefox/133.0";
const BASE_URL: &str = "http://login.scout-gps.ru";

#[derive(Debug)]
pub struct SpicClient {
    client: Client,
    endpoints: SpicUrl
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
            spic_authorization_service: spic_url!("auth"),
            spic_units_service: spic_url!("Units"),
            spic_unit_group_service: spic_url!("UnitGroups"),
            spic_track_periods_mileage_statistics_service: spic_url!("TrackPeriodsMileageStatistics"),
            spic_track_periods_statistics_service: spic_url!("TrackPeriodsStatistics"),
            spic_navigation_filtration_statistics_service: spic_url!("NavigationFiltrationStatistics"),
            spic_navigation_validation_statistics_service: spic_url!("NavigationValidationStatistics"),
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

impl SpicClient {

    fn new(client: Client) -> Self {
        SpicClient {
            client,
            endpoints: SpicUrl::default()
        }
    }

    pub async fn authenticate(&self) -> Result<Response, Box<dyn Error>> {

        // TODO: get credentials from config
        let json_data = json!({
            "Login": "kgm@redlineekb.ru",
            "Password": "5Amxqv",
            "TimeZoneOlsonId": "Asia/Yekaterinburg",
            "CultureName": "ru-ru",
            "UiCultureName": "ru-ru"
        });

        let response =self.client.post(&self.endpoints.spic_authorization_service.0)
        .json(&json_data)
        .send()
        .await;

        match response {
            Ok(response) => Ok(response),
            Err(error) => Err(Box::new(error))
        }
    }
}

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

pub fn init_client() -> SpicClient {

    let mut headers = header::HeaderMap::new();
    headers.insert("Content-Type", header::HeaderValue::from_static("application/json"));
    headers.insert("Accept", header::HeaderValue::from_static("application/json; charset=utf-8"));

    
    let client = Client::builder()
        .user_agent(DEFAULT_USER_AGENT)
        .default_headers(headers)
        .build()
        .expect("Unable to create reqwest client");
    SpicClient::new(client)
}