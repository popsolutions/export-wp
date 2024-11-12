use mysql::{Pool, prelude::*};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};
use reqwest::Client;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::Serialize;
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::task;
use dotenv::dotenv;

// Estrutura para o Token JWT
#[derive(Serialize)]
struct Claims {
    kind: String,
    exp: usize,
    aud: String,
}

// Estrutura para representar um post do WordPress
#[derive(Debug, Serialize)]
struct PostData {
    title: String,
    tags: Vec<String>,
    authors: Vec<String>,
    html: String,
    status: String,
}

fn generate_jwt_token() -> String {
    dotenv().ok();
    let admin_api_key = env::var("ADMIN_API_KEY").unwrap();
    let (id, secret) = admin_api_key.split_once(':').unwrap();
    let header = jsonwebtoken::Header::default();
    let claims = Claims {
        kind: String::from(id),
        exp: (chrono::Utc::now() + chrono::Duration::minutes(5)).timestamp() as usize,
        aud: format!("/admin/"),
    };

    jsonwebtoken::encode(&header, &claims, &jsonwebtoken::EncodingKey::from_secret(secret.as_bytes())).unwrap()
}

async fn send_post(client: Client, post_data: PostData) {
    dotenv().ok();
    let api_url = env::var("API_URL").unwrap();
    let post_api = format!("{}/admin/posts", &api_url);
    let token = generate_jwt_token();
    println!("{ }", &token);
    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, HeaderValue::from_str(&format!("Ghost {}", token)).unwrap());
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
    headers.insert("Accept-Version", HeaderValue::from_static("v3"));


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

// Função principal para migrar os posts
async fn migrate_posts() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    // let db_url = env::var("DB_URL").unwrap();
    // let connection_opts = mysql::Opts::from_url(&db_url).unwrap();    
    // let pool = Pool::new(connection_opts)?;
    // let mut conn = pool.get_conn()?;

    // // Consulta para buscar posts do WordPress
    // let posts: Vec<PostData> = conn
    //     .query_map(
    //         "SELECT post_title, post_name, post_content, post_status, post_date FROM wp_posts WHERE post_type = 'post' AND post_status = 'publish'",
    //         |(title, slug, html, status, published_at)| PostData {
    //             title,
    //             slug,
    //             html,
    //             status,
    //             published_at,
    //         },
    //     )?;

    let client = Client::builder()
        .danger_accept_invalid_certs(true)  // Ignora o certificado SSL inválido        
        .build()?;

    // Envia os posts em paralelo usando `tokio::spawn`
    let mut handles = vec![];
    let title = String::from("Ditadura nunca mais! A verdade sobre a ditadura militar");
    let tag = String::from("ditadura");
    let mut tags = Vec::new();
    tags.push(tag);
    let mut authors = Vec::new();
    authors.push(String::from("joaquin@pop.coop"));
    authors.push(String::from("maquia@pop.coop"));
    let html= String::from("<h1>Abaixo a ditadura</h1>");
    let status = String::from("published");
    let published_at = String::from("2024-11-05T10:00:00.000Z");
    let post = PostData {
        title,
        tags,
        authors,
        html,
        status,
    };
        // for post in posts {
        let client_clone = client.clone();
        let handle = task::spawn(async move {
            send_post(client_clone, post).await;
        });
        handles.push(handle);
    // }

    // Espera que todas as threads terminem
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
