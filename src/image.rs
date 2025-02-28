use anyhow::{Context, Result};
use base64::encode;
use dotenv::dotenv;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};
use reqwest::Client;
use serde::{Serialize, Deserialize};
use std::env;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use tracing::{error, info};
use url::Url;

fn url_to_path(url: &str, root_path: &str) -> Option<String> {
    // Carrega variáveis do arquivo .env
    dotenv().ok();

    // Lê a base URL do ambiente
    let base_url = env::var("DEFAULT_BASE_URL")
        .expect("Failed to get DEFAULT_BASE_URL from env");

    info!("DEFAULT_BASE_URL: {:?}", env::var("DEFAULT_BASE_URL"));
    info!("URL_PATTERNS: {:?}", env::var("URL_PATTERNS"));

    // Lê os padrões de URL do ambiente
    let url_patterns = env::var("URL_PATTERNS").unwrap_or_default();
    info!("url patterns: {:?}", &url_patterns);
    let patterns: Vec<(String, String)> = url_patterns
        .split(',')
        .filter_map(|pair| {
            let parts: Vec<&str> = pair.split('|').collect();
            if parts.len() == 2 {
                Some((parts[0].to_string(), parts[1].to_string()))
            } else {
                None
            }
        })
        .collect();

    info!("Processed patterns: {:?}", patterns);

    // Processa a URL com base nos padrões
    let mut processed_url = url.to_string();
    for (pattern, replacement) in &patterns {
        if processed_url.contains(pattern) {
            processed_url = processed_url.replace(pattern, replacement);
        }
    }

    // Parse da URL processada
    let parsed_url = Url::parse(&processed_url).ok()?;
    info!("Processed URL: {:?}", processed_url);
    info!("Base URL: {:?}", base_url);
    // Constrói o caminho final
    let path = parsed_url.path();
    let full_path = Path::new(root_path).join(path.trim_start_matches('/'));
    full_path.to_str().map(|s| s.to_string())
}

fn image_to_base64(image_path: &str) -> Result<String> {
    let path = Path::new(image_path);

    if !path.exists() {
        error!("File not found: {}", image_path);
        return Err(anyhow::anyhow!("File not found"));
    }

    let mut file =
        File::open(path).with_context(|| format!("Failed to open file: {}", image_path))?;

    let mut buffer = Vec::new();

    file.read_to_end(&mut buffer)
        .with_context(|| format!("Failed to read file: {}", image_path))?;

    let base64_string = encode(&buffer);
    Ok(base64_string)
}


#[derive(Debug, Serialize, Clone)]
pub struct ImageRequest {
    path_image: String,
    base64: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct ImageAuthor {
    author_id: String,
    path_image: String,
    base64: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct ImagePost {
    post_id: String,
    path_image: String,
    base64: String,
}

#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct ImageReply {
    pub image: String,
}

pub async fn send_image(client: Client, image: &str) -> Result<ImageReply> {
    dotenv().ok();
    let token = env::var("API_TOKEN").context("API_TOKEN not found")?;
    let api_url = env::var("API_URL").context("API_URL not found")?;
    let url_req = format!("{}/image", &api_url);

    info!("Processing image URL: {}", image);
    let image_path_res = url_to_path(image, "/var/www/wordpress");
    let post_id = "image basic";

    match image_path_res {
        Some(image_path) => {
            info!("Image path resolved: {}", image_path);
            match image_to_base64(&image_path) {
                Ok(base64) => {
                    let path_image = image_path.replace("/var/www/wordpress", "");
                    let image_post = ImageRequest {
                        path_image: path_image.clone(),
                        base64,
                    };

                    info!("Sending image with path: {}", path_image);
                    let mut headers = HeaderMap::new();
                    headers.insert(
                        AUTHORIZATION,
                        HeaderValue::from_str(&format!("Bearer {}", token)).unwrap(),
                    );
                    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

                    let json_payload = serde_json::to_string(&image_post)?;

                    let res = client.post(&url_req).headers(headers).json(&image_post).send().await;
                    match res {
                        Ok(response) if response.status().is_success() => {
                            info!("Image sent successfully");
                            let image_reply = response.json::<ImageReply>().await?;
                            Ok(image_reply)
                        }
                        Ok(response) => {
                            let status = response.status();
                            let body = response.text().await.unwrap_or_default();
                            error!("Failed to send image: Status {}, Body: {}", status, body);
                            Err(anyhow::anyhow!("Failed to send image"))
                        }
                        Err(e) => {
                            error!("HTTP request error: {}", e);
                            Err(anyhow::anyhow!("HTTP request failed"))
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to encode image to base64: {}", e);
                    Err(anyhow::anyhow!("Failed to encode image"))
                }
            }
        }
        None => {
            error!("Failed to resolve image path");
            Err(anyhow::anyhow!("Failed to resolve image path"))
        }
    }
}

pub async fn send_image_author(client: Client, image: &str, author_id: String) -> Result<ImageReply, String> {
    info!("starting send image from author: { }", &author_id);
    dotenv().ok();
    let token = env::var("API_TOKEN").unwrap();
    let api_url = env::var("API_URL").unwrap();
    let url_req = format!("{}/authors/image", &api_url);
    info!("get path image: { }", &image);
    let image_path_res = url_to_path(image, String::from("/var/www/wordpress").as_str());
    match image_path_res {
        Some(image_path) => match image_to_base64(image_path.as_str()) {
            Ok(base64) => {
                let path_image = image_path.replace("/var/www/wordpress", "");
                let image_post = ImageAuthor {
                    author_id: author_id.clone(),
                    path_image: String::from(path_image),
                    base64,
                };
                info!("send image from author: { }", &author_id);
                let mut headers = HeaderMap::new();
                headers.insert(
                    AUTHORIZATION,
                    HeaderValue::from_str(&format!("Bearer {}", token)).unwrap(),
                );
                headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
                info!("config request");
                let v = serde_json::to_string(&image_post).unwrap();
                let res = client
                    .post(url_req)
                    .headers(headers)
                    .json(&image_post)
                    .send()
                    .await;
                match res {
                    Ok(response) if response.status().is_success() => {
                        info!("request send");
                        println!("Imagem enviada com sucesso: {}", author_id);
                        let image_reply_future = response.json::<ImageReply>().await;
                        match image_reply_future {
                            Ok(image_reply) => {
                                return Ok(image_reply)
                            },
                            Err(err) => {
                                let message_fail = format!("Falha ao enviar imagem: {} - Status: {:?}", author_id, err);
                                return Err(message_fail)
                            }
                        }
                    }
                    Ok(response) if response.status().is_client_error() => {
                        let message_fail = format!("Falha ao enviar imagem: {} - Status: {:?}", author_id, response);
                        error!("request client: {:?}", &message_fail);
                        return Err(message_fail)
                    }
                    Ok(response) if response.status().is_server_error() => {
                        let message_fail = format!("Falha ao enviar imagem: {} - Status: {:?}", author_id, response);
                        error!("request server error: {:?}", &author_id);
                        eprintln!(
                            "Falha ao enviar imagem: {} - Status: {:?}",
                            author_id, response
                        );
                        return Err(message_fail)

                    }

                    Ok(_) => {
                        let message_fail = format!("Falha ao enviar imagem: {}", author_id);
                        error!("request not mapped error: {:?}", &res);
                        return Err(message_fail)

                    }
                    Err(e) => {
                        let message_fail = format!("Falha ao enviar imagem: {} - Status: {:?}", author_id, e);
                        error!("request not mapped error");
                        return Err(message_fail)
                    }
                }
            },
            Err(e) => {
                error!("error file: {:?}", &e);
                let message_fail = format!("Falha ao enviar imagem: {} - Status: {:?}", author_id, e);
                return Err(message_fail) 
            },
        },
        None => {
            error!("image not found");
            let message_fail = format!("Falha ao enviar imagem: {}", author_id);
            return Err(message_fail)
        },
    }
}

pub async fn send_image_post(client: Client, image: &str, post_id: &str) {
    dotenv().ok();
    let token = env::var("API_TOKEN").unwrap();
    let api_url = env::var("API_URL").unwrap();
    let url_req = format!("{}/posts/image", &api_url);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_to_path() {
        // Configura variáveis de ambiente temporárias
        std::env::set_var("DEFAULT_BASE_URL", "www.opiniaosocialista.com.br");
        std::env::set_var("URL_PATTERNS", "https://pstu.org.br");

        // Entrada simulada
        let url = "https://www.pstu.org.br/wp-content/uploads/2019/01/o-PINHEIRINHO-facebook.jpg";
        let root_path = "/var/www/wordpress";

        // Resultado esperado
        let expected_path = Some("/var/www/wordpress/wp-content/uploads/2019/01/o-PINHEIRINHO-facebook.jpg".to_string());

        // Executa a função
        let result = url_to_path(url, root_path);

        // Verifica o resultado
        assert_eq!(result, expected_path);
    }
}
