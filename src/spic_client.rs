use std::error::Error;
use serde::{Deserialize, Serialize};

use reqwest::{Client, Response};

const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:133.0) Gecko/20100101 Firefox/133.0";
const BASE_URL: &str = "http://login.scout-gps.ru";

#[derive(Debug)]
struct SpicClient {
    client: Client,
    endpoints: SpicUrl
}

#[derive(Debug, Deserialize, Serialize)]
struct SpicRequest {
    // Define fields according to the WSDL
    // #[serde(rename = "SomeField")]
    // some_field: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct SpicResponse {
    // Define fields according to the WSDL response
    // #[serde(rename = "SomeResponseField")]
    // some_response_field: String,
}

#[derive(Debug)]
struct Url(String);

macro_rules! spic_url {
    ($expr:expr) => {
        Url(concat!("http://spic.scout-gps.ru/", $expr).to_string())
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

    async fn authenticate(&self) -> Result<Response, Box<dyn Error>> {
        self.client.post(self.endpoints.spic_authorization_service.0).send().await
    }
}

