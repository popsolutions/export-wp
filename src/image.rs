use anyhow::{Context, Result};
use base64::encode;
use dotenv::dotenv;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};
use reqwest::Client;
use serde::Serialize;
use std::env;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use tracing::{error, info};
use url::Url;

fn image_to_base64(image_path: &str) -> Result<String> {
    let path = Path::new(image_path);

    // Abra o arquivo de imagem
    let mut file =
        File::open(path).with_context(|| format!("Failed to open file: {}", image_path))?;

    // Leia o arquivo em um buffer
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .with_context(|| format!("Failed to read file: {}", image_path))?;

    // Codifique os bytes em Base64
    let base64_string = encode(&buffer);

    Ok(base64_string)
}

#[derive(Debug, Serialize, Clone)]
pub struct ImagePost {
    post_id: String,
    base64: String,
}

pub async fn send_image(client: Client, image: &str, post_id: &str) {
    dotenv().ok();
    let token = env::var("API_TOKEN").unwrap();
    let api_url = env::var("API_URL").unwrap();
    let url_req = format!("{}/posts/image", &api_url);
    let image_path_parse = Url::parse(image);
    let image_path_res = image_path_parse.unwrap();

    match image_to_base64(image_path_res.as_str()) {
        Ok(base64) => {
            let image_post = ImagePost {
                post_id: String::from(post_id),
                base64: base64,
            };
            info!("send image from post: { }", &post_id);
            let mut headers = HeaderMap::new();
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {}", token)).unwrap(),
            );
            headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
            info!("config request");
            let v = serde_json::to_string(&image_post).unwrap();
            info!("send post image: {}", v);
            let res = client
                .post(url_req)
                .headers(headers)
                .json(&image_post)
                .send()
                .await;
            match res {
                Ok(response) if response.status().is_success() => {
                    info!("request send");
                    println!("Imagem enviada com sucesso: {}", post_id);
                }
                Ok(response) if response.status().is_client_error() => {
                    error!("request client: {:?}", &post_id);
                    eprintln!(
                        "Falha ao enviar imagem: {} - Status: {:?}",
                        post_id, response
                    );
                }
                Ok(response) if response.status().is_server_error() => {
                    error!("request server error: {:?}", &post_id);
                    eprintln!(
                        "Falha ao enviar imagem: {} - Status: {:?}",
                        post_id, response
                    );
                }

                Ok(_) => {
                    error!("request not mapped error: {:?}", &res);
                }
                Err(e) => {
                    error!("request error: {:?}", &e);
                    eprintln!("Erro ao enviar imagem: {} - Erro: {:?}", &post_id, e);
                }
            }
        }
        Err(e) => error!("error file: {:?}", &e),
    }
}
