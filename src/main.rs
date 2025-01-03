mod config;
mod spic_client;

use std::fs::File;
use std::io::Write;
use chrono::{DateTime, Utc};



use reqwest::{blocking::Client as Client,header, header::{AUTHORIZATION, CONTENT_TYPE}};

fn main() {
    // let config: Config = load_config().expect("Unable to load config file");
    let datetime = Utc::now().to_rfc3339();
    let mut headers = header::HeaderMap::new();
    headers.insert("Content-Type", header::HeaderValue::from_static("application/json"));
    headers.insert("Accept", header::HeaderValue::from_static("application/json; charset=utf-8"));
    // headers.insert("Login", header::HeaderValue::from_static("5Amxqv"));
    // headers.insert("Password", header::HeaderValue::from_static("kgm@redlineekb.ru"));
    // headers.insert("TimeZoneOlsonId", header::HeaderValue::from_static("Europe/Moscow"));
    // headers.insert("CultureName", header::HeaderValue::from_static("ru-ru"));
    // headers.insert("UiCultureName", header::HeaderValue::from_static("ru-ru"));
    // headers.insert("Content-Length", header::HeaderValue::from_static("7"));
    // headers.insert("TimeStampUtc", header::HeaderValue::from_str(&datetime).unwrap());


    let client = Client::builder().
                    user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:133.0) Gecko/20100101 Firefox/133.0").
                    default_headers(headers).
                    build().
                    expect("Unable to create reqwest client");

                    let json_data = r#"{
                        "Login": "kgm@redlineekb.ru",
                        "Password": "5Amxqv",
                        "TimeZoneOlsonId": "Asia/Yekaterinburg",
                        "CultureName": "ru-ru",
                        "UiCultureName": "ru-ru"
                    }"#;

    
    let response = client
        .post("http://login.scout-gps.ru:8081/spic/auth/rest/Login")
        .body(json_data)
        .send();

    let mut file = File::create("response.html").unwrap();
    file.write_all(response.unwrap().text().unwrap().as_bytes()).unwrap();

}

