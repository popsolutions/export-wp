use ammonia::clean;
use dotenv::dotenv;
use mysql::{prelude::*, Pool};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};
use reqwest::Client;
use serde::Serialize;
use std::env;
use tracing::{error, info};

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

#[derive(Debug, Serialize, Clone)]
struct PostData {
    id: u64,
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
            GROUP BY p.ID",
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
        let client_clone = client.clone();
        let handle = tokio::spawn(async move {
            let post_sanitize = post.sanitize();
            send_post(client_clone, post_sanitize).await;
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap();
    }
}
