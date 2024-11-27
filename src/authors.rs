use dotenv::dotenv;
use mysql::{prelude::*, Pool};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};
use reqwest::Client;
use serde::Serialize;
use std::env;
use tokio::task;
use tracing::{error, info};

#[derive(Debug, Serialize)]
struct AuthorData {
    id: i32,
    name: String,
    email: String,
    login: String,
    password: String,
    created_at: String,
}

async fn send_author(client: Client, author_data: AuthorData) {
    dotenv().ok();
    let token = env::var("API_TOKEN").unwrap();
    let api_url = env::var("API_URL").unwrap();
    let url_req = format!("{}/authors", &api_url);
    println!("send author: { }", author_data.name);
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
        .json(&author_data)
        .send()
        .await;

    match res {
        Ok(response) if response.status().is_success() => {
            info!("request send");
            println!("Autor enviado com sucesso: {}", author_data.name);
        }
        Ok(response) if response.status().is_client_error() => {
            error!("request client: {:?}", &author_data);
            eprintln!(
                "Falha ao enviar autor: {} - Status: {:?}",
                author_data.name, response
            );
        }
        Ok(response) if response.status().is_server_error() => {
            error!("request server error: {:?}", &author_data);
            eprintln!(
                "Falha ao enviar autor: {} - Status: {:?}",
                author_data.name, response
            );
        }

        Ok(_) => {
            error!("request not mapped error: {:?}", &res);
        }
        Err(e) => {
            error!("request error: {:?}", &e);
            eprintln!("Erro ao enviar autor: {} - Erro: {:?}", author_data.name, e);
        }
    }
}

pub async fn migrate_authors() {
    dotenv().ok();
    let db_url = env::var("DB_URL").unwrap();
    let connection_opts = mysql::Opts::from_url(&db_url).unwrap();
    let pool = Pool::new(connection_opts).unwrap();
    let mut conn = pool.get_conn().unwrap();

    let authors: Vec<AuthorData> = conn
        .query_map(
            "SELECT DISTINCT
                    u.ID AS id,
                    u.user_nicename AS name,
                    u.user_email AS email,
                    u.user_login AS login,
                    u.user_pass AS password,
                    u.user_registered AS created_at
                FROM
                    wp_users u
                JOIN
                    wp_posts p ON u.ID = p.post_author
                WHERE
                    p.post_type = 'post' AND
                    p.post_status = 'publish'",
            |(id, name, email, login, password, created_at)| AuthorData {
                id,
                name,
                email,
                login,
                password,
                created_at,
            },
        )
        .unwrap();
    info!("ok query authors");
    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();
    let mut handles = vec![];
    for author in authors {
        let client_clone = client.clone();
        let handle = task::spawn(async move {
            send_author(client_clone, author).await;
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }
}
