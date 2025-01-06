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

const DEFAULT_BASE_URL: &str = "http://www.pstu.org.br";

fn url_to_path(url: &str, root_path: &str) -> Option<String> {
    // Tenta parsear a URL fornecida
    let parsed_url = Url::parse(url).ok()?;
    let base_url = Url::parse(DEFAULT_BASE_URL).ok()?;

    // Verifica se a URL fornecida tem a mesma origem que a base URL padrão
    if parsed_url.host_str()? != base_url.host_str()? {
        return None;
    }

    // Obtém o caminho da URL e junta com o caminho raiz fornecido
    let path = parsed_url.path();
    let full_path = Path::new(root_path).join(path.trim_start_matches('/'));

    // Retorna o path como String
    full_path.to_str().map(|s| s.to_string())
}

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
    path_image: String,
    base64: String,
}

pub async fn send_image(client: Client, image: &str, post_id: &str) {
    dotenv().ok();
    let token = env::var("API_TOKEN").unwrap();
    let api_url = env::var("API_URL").unwrap();
    let url_req = format!("{}/image", &api_url);
    let image_path_res = url_to_path(image, String::from("/var/www/wordpress").as_str());
    match image_path_res {
        Some(image_path) => match image_to_base64(image_path.as_str()) {
            Ok(base64) => {
                let path_image = image_path.replace("/var/www/wordpress", "");
                let image_post = ImagePost {
                    post_id: String::from(post_id),
                    path_image: String::from(path_image),
                    base64,
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
        },
        None => {
            error!("image not found");
        }
    }
}
