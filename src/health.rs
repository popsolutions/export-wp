use ammonia::clean;
use dotenv::dotenv;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};
use reqwest::Client;
use reqwest::tls::Version;
use serde::{Deserialize, Serialize};
use std::env;
use tracing::{error, info};
use mysql::{prelude::*, Pool, Opts};

use crate::image::send_image;

pub async fn test_db_connection() -> Result<(), Box<dyn std::error::Error>> {
    if dotenv().is_err() {
        error!("Arquivo .env não encontrado ou inválido.");
        return Err("Arquivo .env não encontrado ou inválido.".into());
    }

    // Tenta obter a variável de ambiente DB_URL
    let db_url = match env::var("DB_URL") {
        Ok(url) => url,
        Err(_) => {
            error!("Variável de ambiente DB_URL não encontrada.");
            return Err("Variável de ambiente DB_URL não encontrada.".into());
        }
    };

    let db_url = match env::var("API_URL") {
        Ok(url) => url,
        Err(_) => {
            error!("Variável de ambiente API_URL não encontrada.");
            return Err("Variável de ambiente API_URL não encontrada.".into());
        }
    };

    let db_url = match env::var("API_TOKEN") {
        Ok(url) => url,
        Err(_) => {
            error!("Variável de ambiente API_TOKEN não encontrada.");
            return Err("Variável de ambiente API_TOKEN não encontrada.".into());
        }
    };

    // Tenta criar as opções de conexão com o banco de dados
    let connection_opts = match Opts::from_url(&db_url) {
        Ok(opts) => opts,
        Err(e) => {
            error!("Falha ao criar opções de conexão: {:?}", e);
            return Err(e.into());
        }
    };

    // Tenta criar o pool de conexões
    let pool = match Pool::new(connection_opts) {
        Ok(pool) => pool,
        Err(e) => {
            error!("Falha ao criar o pool de conexões: {:?}", e);
            return Err(e.into());
        }
    };

    // Tenta obter uma conexão do pool
    match pool.get_conn() {
        Ok(_) => {
            info!("Conexão com o banco de dados estabelecida com sucesso.");
            Ok(())
        }
        Err(e) => {
            error!("Falha ao conectar ao banco de dados: {:?}", e);
            Err(e.into())
        }
    }
}

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
