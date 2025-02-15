use ammonia::clean;
use dotenv::dotenv;
use mysql::{prelude::*, Pool};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};
use reqwest::Client;
use reqwest::tls::Version;
use serde::{Deserialize, Serialize};
use std::env;
use tracing::{error, info};

use crate::image::send_image;

pub async fn heathcheck() {
    dotenv().ok();
    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .min_tls_version(Version::TLS_1_2)
        .build()
        .unwrap();
    let token = env::var("API_TOKEN").unwrap();
    let api_url = env::var("API_URL").unwrap();
    let url_req = format!("{}/healthcheck", &api_url);
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", token)).unwrap(),
    );
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
    info!("config request");
    let res = client.get(url_req).headers(headers).send().await;

    match res {
        Ok(response) => info!("Connection ok: {:?}", response),
        Err(e) => error!("Fail to connect: {:?}", e),
    }
}
