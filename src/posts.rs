use crate::image::{send_image, send_image_post};
use ammonia::clean;
use anyhow::{Context, Result};
use dotenv::dotenv;
use mockall::predicate::*;
use mysql::{prelude::*, Pool};
use regex::Regex;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};
use reqwest::tls::Version;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use tokio;
use tokio::task::JoinHandle;
use tracing::{error, info};

#[derive(Debug, Deserialize, Serialize)]
pub struct PostReply {
    id: String,
    title: String,
    slug: String,
    created_at: String,
    updated_at: String,
    author_id: String,
}

async fn send_post(client: Client, post_data: PostData) -> Option<PostReply> {
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
            let data_res: PostReply = response.json().await.unwrap();
            Some(data_res)
        }
        Ok(response) if response.status().is_client_error() => {
            error!("request client: {:?}", &post_data);
            eprintln!(
                "Falha ao enviar post: {} - Status: {:?}",
                post_data.title, response
            );
            None
        }
        Ok(response) if response.status().is_server_error() => {
            error!("request server error: {:?}", &post_data);
            eprintln!(
                "Falha ao enviar post: {} - Status: {:?}",
                post_data.title, response
            );
            None
        }

        Ok(_) => {
            error!("request not mapped error: {:?}", &res);
            None
        }
        Err(e) => {
            error!("request error: {:?}", &e);
            eprintln!("Erro ao enviar post: {} - Erro: {:?}", post_data.title, e);
            None
        }
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct PostData {
    pub id: u64,
    title: String,
    slug: String,
    html: String,
    created_at: String,
    updated_at: String,
    author_id: String,
    image_url: Option<String>,
    tags: Option<String>,
}

fn text_to_html_paragraphs(input: &str) -> String {
    input
        .split("\n\n") // Divide o texto em partes por \n\n
        .map(|paragraph| format!("<p>{}</p>", paragraph.trim())) // Cria os parágrafos
        .collect::<Vec<String>>() // Coleta os parágrafos em um vetor
        .join("\n") // Junta os parágrafos com uma quebra de linha
}

impl PostData {
    fn sanitize(self, content: String) -> Self {
        let content = text_to_html_paragraphs(&content);

        Self {
            html: clean(&content),
            ..self
        }
    }
}

async fn get_posts() -> Result<Vec<PostData>, String> {
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

    let result_query_posts = conn.query_map(
        r#"
            SELECT
        p.ID AS id,
        p.post_title AS title,
        p.post_name AS slug,
        p.post_content AS html,
        p.post_date AS created_at,
        p.post_modified AS updated_at,
        p.post_author AS author_id,
        SUBSTRING_INDEX(
            SUBSTRING_INDEX(
                SUBSTRING(p.post_content, LOCATE('<img', p.post_content)),
                'src="', -1),
                    '"', 1) AS image_url,
                GROUP_CONCAT(t.name) AS tags
            FROM
                wp_posts p
            INNER JOIN
                wp_term_relationships tr ON p.ID = tr.object_id
            INNER JOIN
                wp_term_taxonomy tt ON tr.term_taxonomy_id = tt.term_taxonomy_id
            INNER JOIN
                wp_terms t ON tt.term_id = t.term_id
            WHERE
                p.post_type = 'post'
                AND p.post_status = 'publish'
                AND tt.taxonomy = 'category'
                AND p.post_content LIKE '%<img%'
            GROUP BY
                p.ID;"#,
        |(id, title, slug, html, created_at, updated_at, author_id, image_url, tags)| PostData {
            id,
            title,
            slug,
            html,
            created_at,
            updated_at,
            author_id,
            image_url,
            tags,
        },
    );

    match result_query_posts {
        Ok(res) => {
            let posts: Vec<PostData> = res;
            info!("ok query posts");
            return Ok(posts);
        }
        Err(message) => {
            error!("Fail to query posts: {}", message);
            Err(String::from("fail to query posts"))
        }
    }
}

pub async fn process_html(html: String, client: Client) -> String {
    let regex_image = match Regex::new(r#"<img[^>]+src="([^">]+)"#) {
        Ok(regex) => regex,
        Err(err) => {
            error!("Failed to compile regex: {:?}", err);
            return html; // Retorna o HTML original se a regex falhar
        }
    };

    let mut processed_html = html.clone();
    for cap in regex_image.captures_iter(&html) {
        let image_url = match cap.get(1) {
            Some(url) => url.as_str(),
            None => continue,
        };

        match send_image(client.clone(), image_url).await {
            Ok(image_reply_ok) => {
                let new_url = format!("__GHOST_URL__{}", image_reply_ok.image);
                processed_html = processed_html.replace(image_url, &new_url);
            }
            Err(err) => {
                error!("Failed to send image: {:?}", err);
                // Continua o processamento mesmo se uma imagem falhar
            }
        }
    }

    processed_html
}

async fn process_post(client: Client, post: PostData)  {
    let client_clone_image = client.clone();
    let client_clone_thumb = client.clone();
    let client_clone_post = client.clone();

    let handle = tokio::spawn(async move {
        let mut post_clone = post.clone();

        let processed_html = process_html(post.html.to_string(), client_clone_image).await;
        let post_sanitize = post.sanitize(processed_html);

        if let Some(post_saved) = send_post(client_clone_post, post_sanitize).await {
            info!("Post reply received: {:?}", &post_saved);
            info!("Post reply received: {:?}", &post_saved);
            info!("Post clone: {:?}", &post_clone);
            if let Some(image_url) = post_clone.image_url {
                send_image_post(client_clone_thumb, &image_url, &post_saved.id).await;
                info!("Image sent");
            } else {
                info!("No image found")
            }
        } else {
            error!("No post reply received");
        }
    });
    if let Err(err) = handle.await {
       error!("Task failed: {:?}", err);
    }
}

pub async fn migrate_posts() {
    match get_posts().await {
        Ok(posts) => {
            info!("found {} posts from database", posts.len());
            let client = Client::builder()
                .min_tls_version(Version::TLS_1_2)
                .danger_accept_invalid_certs(true)
                .build()
                .unwrap();
            let mut handles = vec![];
            for post in posts {
                let client_clone = client.clone();
                let handle = tokio::spawn(async move {
                    process_post(client_clone, post).await;
                });
                handles.push(handle);
            }
            // Aguarda a conclusão da tarefa
            for handle in handles {
                if let Err(err) = handle.await {
                    error!("Fail to send process_migrate_post: {:?}", err);
                }
            }
        }
        Err(message) => {
            error!("Posts not found: {:?}", message);
        }
    }
}
