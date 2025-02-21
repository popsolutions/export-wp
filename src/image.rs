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
    dotenv().ok();
    let base_url = env::var("DEFAULT_BASE_URL")
        .context("Failed to get DEFAULT_BASE_URL from env")
        .unwrap()
        .to_string();  
    info!("base_url from env: {}", base_url); 
    let parsed_url = Url::parse(url).ok()?;
    info!("parse image: {}", parsed_url);
    info!("parse image host: {}", parsed_url.host_str()?);

    if parsed_url.host_str()? != base_url {
        return None;
    }

    let path = parsed_url.path();
    let full_path = Path::new(root_path).join(path.trim_start_matches('/'));

    full_path.to_str().map(|s| s.to_string())
}

fn image_to_base64(image_path: &str) -> Result<String> {
    let path = Path::new(image_path);

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

pub async fn send_image(client: Client, image: &str) {
    dotenv().ok();
    let token = env::var("API_TOKEN").unwrap();
    let api_url = env::var("API_URL").unwrap();
    let url_req = format!("{}/image", &api_url);
    let image_path_res = url_to_path(image, String::from("/var/www/wordpress").as_str());
    match image_path_res {
        Some(image_path) => match image_to_base64(image_path.as_str()) {
            Ok(base64) => {
                let path_image = image_path.replace("/var/www/wordpress", "");
                let image_post = ImageRequest {
                    path_image: String::from(path_image),
                    base64,
                };
                info!("send image");
                let mut headers = HeaderMap::new();
                headers.insert(
                    AUTHORIZATION,
                    HeaderValue::from_str(&format!("Bearer {}", token)).unwrap(),
                );
                headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
                info!("config request");
                let v = serde_json::to_string(&image_post).unwrap();
                info!("send image: {}", v);
                let res = client
                    .post(url_req)
                    .headers(headers)
                    .json(&image_post)
                    .send()
                    .await;
                match res {
                    Ok(response) if response.status().is_success() => {
                        info!("request send");
                        println!("Imagem enviada com sucesso");
                    }
                    Ok(response) if response.status().is_client_error() => {
                        error!("request client");
                        eprintln!(
                            "Falha ao enviar imagem: Status: {:?}", response
                        );
                    }
                    Ok(response) if response.status().is_server_error() => {
                        error!("request server error");
                        eprintln!(
                            "Falha ao enviar imagem: Status: {:?}",
                            response
                        );
                    }

                    Ok(_) => {
                        error!("request not mapped error: {:?}", &res);
                    }
                    Err(e) => {
                        error!("request error: {:?}", &e);
                        eprintln!("Erro ao enviar imagem: - Erro: {:?}", e);
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
