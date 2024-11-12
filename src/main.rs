use mysql::{Pool, prelude::*};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};
use reqwest::Client;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::Serialize;
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::task;
use dotenv::dotenv;

// Estrutura para representar um post do WordPress
#[derive(Debug, Serialize)]
struct PostData {
    title: String,
    tags: Vec<String>,
    authors: Vec<String>,
    html: String,
    status: String,
}

async fn send_post(client: Client, post_data: PostData) {
    dotenv().ok();
    let token = "";
    println!("send post: { }", post_data.title);
    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", token)).unwrap());
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));    

    let res = client
        .post(post_api)
        .headers(headers)
        .json(&serde_json::json!({ "posts": [post_data] }))
        .send()
        .await;

    match res {
        Ok(response) if response.status().is_success() => {
            println!("Post enviado com sucesso: {}", post_data.title);
        }
        Ok(response) => {
            eprintln!("Falha ao enviar post: {} - Status: {:?}", post_data.title, response);
        }
        Err(e) => {
            eprintln!("Erro ao enviar post: {} - Erro: {:?}", post_data.title, e);
        }
    }
}

async fn migrate_posts() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let db_url = env::var("DB_URL").unwrap();
    let connection_opts = mysql::Opts::from_url(&db_url).unwrap();    
    let pool = Pool::new(connection_opts)?;
    let mut conn = pool.get_conn()?;

    let posts: Vec<PostData> = conn
        .query_map(
            "SELECT post_title, post_name, post_content, post_status, post_date FROM wp_posts WHERE post_type = 'post' AND post_status = 'publish'",
            |(title, slug, html, status, published_at)| PostData {
                title,
                slug,
                html,
                status,
                published_at,
            },
        ).unwrap();

    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();

    for post in posts {
        let client_clone = client.clone();
        let handle = task::spawn(async move {
            send_post(client_clone, post).await;
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
    if let Err(e) = migrate_posts().await {
        eprintln!("Erro durante a migração: {:?}", e);
    }
}
