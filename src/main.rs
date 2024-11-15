use mysql::{Pool, prelude::*};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};
use reqwest::Client;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::Serialize;
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::task;
use dotenv::dotenv;

#[derive(Debug, Serialize)]
struct AuthorData {    
    name: String,
    email: String,
}

async fn send_author(client: Client, author_data: AuthorData) {
    dotenv().ok();
    let token = env::var("API_TOKEN").unwrap();
    let api_url = env::var("API_URL").unwrap();
    let url_req = format!("{}/authors", &api_url);
    println!("send author: { }", author_data.name);
    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", token)).unwrap());
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));    

    let res = client
        .post(url_req)
        .headers(headers)
        .json(&serde_json::json!({ "authors": [author_data] }))
        .send()
        .await;

    match res {
        Ok(response) if response.status().is_success() => {
            println!("Autor enviado com sucesso: {}", author_data.name);
        }
        Ok(response) => {
            eprintln!("Falha ao enviar autor: {} - Status: {:?}", author_data.name, response);
        }
        Err(e) => {
            eprintln!("Erro ao enviar autor: {} - Erro: {:?}", author_data.name, e);
        }
    }
}

async fn migrate_authors() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let db_url = env::var("DB_URL").unwrap();
    let connection_opts = mysql::Opts::from_url(&db_url).unwrap();    
    let pool = Pool::new(connection_opts)?;
    let mut conn = pool.get_conn()?;

    let authors: Vec<AuthorData> = conn
        .query_map(
            "SELECT DISTINCT
                    u.ID AS autor_id,
                    u.user_login AS login,
                    u.user_nicename AS name,
                    u.user_email AS email,
                    u.display_name AS display_name
                FROM
                    wp_users u
                JOIN
                    wp_posts p ON u.ID = p.post_author
                WHERE
                    p.post_type = 'post' AND
                    p.post_status = 'publish'",
            |( name, email)| AuthorData {
                name,
                email,                
            },
        ).unwrap();

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
        handle.await?;
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(e) = migrate_authors().await {
        eprintln!("Erro durante a migração: {:?}", e);
    }
}
