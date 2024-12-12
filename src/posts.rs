use ammonia::clean;
use dotenv::dotenv;
use mysql::{prelude::*, Pool};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use tracing::{error, info};

use crate::image::send_image;

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
    pub image_url: Option<String>,
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
    fn sanitize(self) -> Self {
        let content = text_to_html_paragraphs(&self.html);
        Self {
            html: clean(&content),
            ..self
        }
    }
}

pub async fn migrate_posts() {
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
                p.post_author AS author_id,
                img.guid AS image_url,
                GROUP_CONCAT(t.name) AS tags
            FROM
                wp_posts p
            LEFT JOIN
                wp_posts img ON img.post_parent = p.ID AND img.post_type = 'attachment' AND img.post_mime_type LIKE 'image/%'
            INNER JOIN
                wp_term_relationships tr ON p.ID = tr.object_id
            INNER JOIN
                wp_term_taxonomy tt ON tr.term_taxonomy_id = tt.term_taxonomy_id
            INNER JOIN
                wp_terms t ON tt.term_id = t.term_id
            WHERE
                p.post_type = 'post'
                AND p.post_status = 'publish'
                AND p.post_type = 'post'
                AND tt.taxonomy = 'category'
            GROUP BY p.ID limit 1",
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
        )
        .unwrap();

    info!("ok query posts");
    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();
    let mut handles = vec![];
    for post in posts {
        let client_clone_image = client.clone();
        let client_clone_post = client.clone();
        let handle = tokio::spawn(async move {
            let post_sanitize = post.sanitize();
            let post_reply = send_post(client_clone_post, post_sanitize).await;
            if let Some(post_saved) = post_reply {
                info!("Post reply received: {:?}", &post_saved);

                if let Some(image) = &post.image_url {
                    send_image(client_clone_image, image, post_saved.id).await;
                    info!("Image sent");
                } else {
                    info!("No image found");
                }
            } else {
                info!("No post reply received");
            }

            match &post.image_url {
                Some(image) => {
                    send_image(client_clone_image, image, post.id).await;
                    info!("image sended");
                }
                None => {
                    info!("No image found");
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }
}
