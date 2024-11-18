use ammonia::clean;
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

async fn migrate_authors() {
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

#[derive(Debug, Serialize, Clone)]
struct PostData {
    id: u64,
    title: String,
    slug: String,
    html: String,
    created_at: String,
    updated_at: String,
    author_id: u64,
}

fn text_to_html_paragraphs(input: &str) -> String {
    input
        .split("\n\n") // Divide o texto em partes por \n\n
        .map(|paragraph| format!("<p>{}</p>", paragraph.trim())) // Cria os parágrafos
        .collect::<Vec<String>>() // Coleta os parágrafos em um vetor
        .join("\n") // Junta os parágrafos com uma quebra de linha
}

impl PostData {
    fn sanitize(self) -> Self {
        let content = text_to_html_paragraphs(&self.html);
        Self {
            html: clean(&content),
            ..self
        }
    }
}

#[derive(Debug, Serialize)]
struct TagData {
    id: i32,
    name: String,
    slug: String,
}

async fn send_post(client: Client, post_data: PostData) {
    dotenv().ok();
    let token = env::var("API_TOKEN").unwrap();
    let api_url = env::var("API_URL").unwrap();
    let url_req = format!("{}/posts", &api_url);
    info!("send post: { }", post_data.title);
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", token)).unwrap(),
    );
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
    info!("config request");
    let v = serde_json::to_string(&post_data).unwrap();
    info!("send post: {}", v);
    let res = client
        .post(url_req)
        .headers(headers)
        .json(&post_data)
        .send()
        .await;

    match res {
        Ok(response) if response.status().is_success() => {
            info!("request send");
            println!("Post enviado com sucesso: {}", post_data.title);
        }
        Ok(response) if response.status().is_client_error() => {
            error!("request client: {:?}", &post_data);
            eprintln!(
                "Falha ao enviar post: {} - Status: {:?}",
                post_data.title, response
            );
        }
        Ok(response) if response.status().is_server_error() => {
            error!("request server error: {:?}", &post_data);
            eprintln!(
                "Falha ao enviar post: {} - Status: {:?}",
                post_data.title, response
            );
        }

        Ok(_) => {
            error!("request not mapped error: {:?}", &res);
        }
        Err(e) => {
            error!("request error: {:?}", &e);
            eprintln!("Erro ao enviar post: {} - Erro: {:?}", post_data.title, e);
        }
    }
}

async fn migrate_posts() {
    dotenv().ok();
    let db_url = env::var("DB_URL").unwrap();
    let connection_opts = mysql::Opts::from_url(&db_url).unwrap();
    let pool = Pool::new(connection_opts).unwrap();
    let mut conn = pool.get_conn().unwrap();

    let posts: Vec<PostData> = conn
        .query_map(
            "SELECT
                 p.ID AS id,
                 p.post_title AS title,
                 p.post_name AS slug,
                 p.post_content AS html,
                 p.post_date AS created_at,
                 p.post_modified AS updated_at,
                 p.post_author AS author_id
             FROM
                 wp_posts p
             WHERE
                 p.post_type = 'post' AND
                 p.post_status = 'publish'",
            |(id, title, slug, html, created_at, updated_at, author_id)| PostData {
                id,
                title,
                slug,
                html,
                created_at,
                updated_at,
                author_id,
            },
        )
        .unwrap();

    info!("ok query posts");
    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();
    let mut handles = vec![];
    // for post in posts {
    let client_clone = client.clone();
    let handle = tokio::spawn(async move {
        let post = posts.into_iter().nth(0).unwrap();
        let post_sanitize = post.sanitize();
        send_post(client_clone, post_sanitize).await;
    });
    handles.push(handle);
    // }

    for handle in handles {
        handle.await.unwrap();
    }
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

async fn migrate_tags() {
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
                 tt.taxonomy = 'post_tag'",
            |(id, name, slug)| TagData { id, name, slug },
        )
        .unwrap();

    info!("ok query tags");
    let client = Client::builder()
        .danger_accept_invalid_certs(true)
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

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // migrate_authors().await;
    // migrate_tags().await;
    migrate_posts().await;
}
