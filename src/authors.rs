use anyhow::{Context, Result};
use dotenv::dotenv;
use killer::process_image_url;
use mysql::{prelude::*, Pool};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};
use reqwest::tls::Version;
use reqwest::Client;
use serde::Serialize;
use std::env;
use tokio::task;
use tracing::{error, info};

#[derive(Debug, Serialize)]
struct AuthorPost {
    id: i32,
    name: String,
    email: String,
    login: String,
    password: String,
    created_at: String,
    image_url: Option<String>,
}

#[derive(Debug, Serialize)]
struct ProfileImage {
    author_id: String,
    path_image: String,
    base64: String,
}

#[derive(Debug, Serialize)]
struct AuthorRequest {
    id: i32,
    name: String,
    email: String,
    login: String,
    password: String,
    created_at: String,
}

impl AuthorPost {
    fn update_image(self, image: String) -> Self {
        Self {
            image_url: Some(image),
            ..self
        }
    }
}

async fn send_author(client: Client, author_data: AuthorPost) -> Result<(), String> {
    dotenv().ok();
    let token = env::var("API_TOKEN")
        .context("Failed to get API_TOKEN from env")
        .unwrap();
    let api_url = env::var("API_URL")
        .context("Failed to get API_URL from env")
        .unwrap();
    let url_req = format!("{}/authors", &api_url);
    println!("send author: { }", author_data.name);

    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", token))
            .context("Failed to create authorization header")
            .unwrap(),
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
            Ok(())
        }
        Ok(response) if response.status().is_client_error() => {
            error!("request client: {:?}", &author_data);
            eprintln!(
                "Falha ao enviar autor: {} - Status: {:?}",
                author_data.name, response
            );
            Err(String::from("Fail to env"))
        }
        Ok(response) if response.status().is_server_error() => {
            error!("request server error: {:?}", &author_data);
            eprintln!(
                "Falha ao enviar autor: {} - Status: {:?}",
                author_data.name, response
            );
            Err(String::from("Fail to env"))
        }

        Ok(_) => {
            error!("request not mapped error: {:?}", &res);
            Err(String::from("request not mapped"))
        }
        Err(e) => {
            error!("request error: {:?}", &e);
            eprintln!("Erro ao enviar autor: {} - Erro: {:?}", author_data.name, e);
            Err(String::from("error request"))
        }
    }
}

async fn get_authors() -> Result<Vec<AuthorPost>, String> {
    dotenv().ok();
    let db_url = env::var("DB_URL")
        .context("Failed to get DB_URL from env")
        .unwrap();
    let connection_opts = mysql::Opts::from_url(&db_url)
        .context("Failed to parse DB_URL")
        .unwrap();
    let pool = Pool::new(connection_opts)
        .context("Failed to create connection pool")
        .unwrap();
    let mut conn = pool
        .get_conn()
        .context("Failed to get connection from pool")
        .unwrap();

    let result_query_authors = conn
        .query_map(
            "SELECT DISTINCT
                    u.ID AS id,
                    u.display_name AS name,
                    u.user_email AS email,
                    u.user_login AS login,
                    u.user_pass AS password,
                    u.user_registered AS created_at,
                    (SELECT meta_value FROM wp_usermeta WHERE user_id = u.ID AND meta_key = 'molongui_author_image_url') AS profile_image_url
                FROM
                    wp_users u
                JOIN
                    wp_posts p ON u.ID = p.post_author
                LEFT JOIN
                    wp_usermeta um ON u.ID = um.user_id
                WHERE
                    p.post_type = 'post' AND
                    p.post_status = 'publish' AND
                    u.user_email IS NOT NULL AND
                    u.user_email <> ''
                ",
            |(id, name, email, login, password, created_at, profile_image_url): (i32, String, String, String, String, String, Option<String>)|
            AuthorPost {
                id,
                name,
                email,
                login,
                password,
                created_at,
                image_url: profile_image_url,
            },
        );
    match result_query_authors {
        Ok(res) => {
            let authors: Vec<AuthorPost> = res;
            info!("ok query authors");
            return Ok(authors);
        }
        Err(message) => {
            error!("Fail to query author: {}", message);
            Err(String::from("fail to query author"))
        }
    }
}

pub async fn migrate_authors() {
    match get_authors().await {
        Ok(authors) => {
            info!("found {} authors from database", authors.len());
            let client = Client::builder()
                .min_tls_version(Version::TLS_1_2)
                .danger_accept_invalid_certs(true)
                .build()
                .unwrap();
            let mut handles = vec![];

            for author in authors {
                let client_clone = client.clone();

                let handle = task::spawn(async move {
                    let image_right = if let Some(image_url) = &author.image_url {
                        process_image_url(image_url)
                    } else {
                        String::from("")
                    };
                    let author_change = AuthorPost {
                        image_url: Some(image_right),
                        ..author
                    };

                    match send_author(client_clone, author_change).await {
                        Ok(_) => {
                            info!("Author updated successfully");
                        }
                        Err(e) => {
                            error!("Failed to update author {:?}", e);
                        }
                    }
                });

                handles.push(handle);
            }

            // Aguardar a conclusão de todas as tarefas
            for handle in handles {
                if let Err(e) = handle.await {
                    error!("Task failed: {:?}", e);
                }
            }
        }
        Err(message) => {
            error!("Authors not found: {:?}", message);
        }
    }
}
