use dotenv::dotenv;
use mysql::{prelude::*, Pool};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};
use reqwest::Client;
use reqwest::tls::Version;
use serde::Serialize;
use std::env;
use tracing::{error, info};

#[derive(Debug, Serialize)]
struct TagData {
    id: i32,
    name: String,
    slug: String,
}

async fn send_tag(client: Client, tag: TagData) {
    let token = env::var("API_TOKEN").unwrap();
    let api_url = env::var("API_URL").unwrap();
    let url_req = format!("{}/tags", &api_url);
    info!("send tag: { }", tag.name);
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", token)).unwrap(),
    );
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
    info!("config request");
    let res = client
        .post(url_req)
        .headers(headers)
        .json(&tag)
        .send()
        .await;

    match res {
        Ok(response) => info!("Tag enviada com sucesso: {:?}", response),
        Err(e) => error!("Erro ao enviar tag: {:?}", e),
    }
}

pub async fn migrate_tags() {
    dotenv().ok();
    let db_url = env::var("DB_URL").unwrap();
    let connection_opts = mysql::Opts::from_url(&db_url).unwrap();
    let pool = Pool::new(connection_opts).unwrap();
    let mut conn = pool.get_conn().unwrap();

    let tags: Vec<TagData> = conn
        .query_map(
            "SELECT
                 t.term_id AS id,
                 t.name AS name,
                 t.slug AS slug
             FROM
                 wp_terms t
             JOIN
                 wp_term_taxonomy tt ON t.term_id = tt.term_id
             WHERE
                 tt.taxonomy = 'category'",
            |(id, name, slug)| TagData { id, name, slug },
        )
        .unwrap();

    info!("ok query tags");
    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .min_tls_version(Version::TLS_1_2)
        .build()
        .unwrap();
    let mut handles = vec![];
    for tag in tags {
        let client_clone = client.clone();
        let handle = tokio::spawn(async move {
            send_tag(client_clone, tag).await;
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }
}
