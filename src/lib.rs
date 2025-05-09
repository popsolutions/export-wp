use lazy_static::lazy_static;
use rand::Rng;
use regex::Regex;

pub fn process_image_url(image_url: &str) -> String {
    if let Some(pos) = image_url.find("/wp-content/") {
        // Concatenate the base URL with everything after "/wp-content/"
        let new_format = format!("/{}", &image_url[pos + 4..]);
        format!(
            "{}",
            new_format.replace("/content/uploads", "/content/images")
        )
    } else {
        // YYYY/MM/name.file
        format!("/content/images/{}", image_url)
    }
}



lazy_static! {
    // Regex para capturar cabeçalhos, captions e imagens
    static ref BLOCKS: Regex = Regex::new(
        r"(?s)(<h[1-6][^>]*>.*?</h[1-6]>|\[caption[^\]]*\].*?\[/caption\]|<img[^>]+>)"
    ).unwrap();
    // Regex para identificar múltiplas quebras de linha
    static ref MULTI_NEWLINE: Regex = Regex::new(r"\n\s*\n").unwrap();
    // Regex para extrair informações do caption
    static ref CAPTION: Regex = Regex::new(
        r#"\[caption[^\]]*id="attachment_(\d+)"[^\]]*\].*?<img[^>]+class="[^"]*wp-image-(\d+)[^"]*"[^>]+src="([^"]+)"[^>]+alt="([^"]*)"[^>]*>([^\[]*)\[/caption\]"#
    ).unwrap();
}

pub fn text_to_html_paragraphs(text: &str) -> String {
    let mut result = String::new();
    let mut last_end = 0;
    let mut p_count = 0;
    let mut ad_inserted = false;
    
    // Encontra todos os blocos especiais
    for cap in BLOCKS.captures_iter(text) {
        let m = cap.get(0).unwrap();

        // Processa o texto antes do bloco especial
        let before_block = &text[last_end..m.start()];
        if !before_block.trim().is_empty() {
            let (processed, count) = wrap_and_count_paragraphs(before_block);
            result.push_str(&processed);
            p_count += count;
            
            // Insere anúncio antes do 4º parágrafo
            if p_count >= 3 && count > 0 && !ad_inserted {
                insert_advertisement(&mut result, &mut p_count);
                ad_inserted = true;
            }

        }

        // Processa o bloco especial
        let block = m.as_str();
        if block.starts_with("[caption") {
            // Transforma o caption em HTML moderno
            result.push_str(&process_caption(block));
        } else {
            // Mantém outros blocos como estão
            result.push_str(block);
        }

        last_end = m.end();
    }

    // Processa o texto restante após o último bloco especial
    let remaining = &text[last_end..];
    if !remaining.trim().is_empty() {
        let (processed, count) = wrap_and_count_paragraphs(remaining);
        result.push_str(&processed);
        p_count += count;
    }


    result
}

fn process_caption(caption: &str) -> String {
    if let Some(caps) = CAPTION.captures(caption) {
        let id = caps.get(1).map_or("", |m| m.as_str());
        let src = caps.get(3).map_or("", |m| m.as_str());
        let alt = caps.get(4).map_or("", |m| m.as_str());
        let description = caps.get(5).map_or("", |m| m.as_str()).trim();

        format!(r#"<figure id="attachment_{}"><picture><img src="{}" alt="{}" loading="lazy"></picture><figcaption>{}</figcaption></figure>"#, id, src, alt, description)
    } else {
        // Se não conseguir parsear, retorna o original
        caption.to_string()
    }
}

// Função auxiliar para envolver texto em parágrafos
fn wrap_and_count_paragraphs(text: &str) -> (String, usize) {
    let mut result = String::new();
    let paragraphs: Vec<&str> = MULTI_NEWLINE.split(text).collect();
    let mut count = 0;

    for paragraph in paragraphs {
        let trimmed = paragraph.trim();
        if !trimmed.is_empty() {
            result.push_str(&format!("<p>{}</p>", trimmed));
            count += 1;
        }
    }

    (result, count)
}

fn insert_advertisement(result: &mut String, p_count: &mut usize) {
    let mut rng = rand::thread_rng();
    let random_number = rng.gen_range(1..=3);
    
    let ad = format!(
        r#"<a href="/nos-apoie" class="post-anuncio"><img src="/content/images/2025/05/contribua_v{}.jpg" alt="contribua com o opinião socialista" loading="lazy"></a>"#,
        random_number
    );
    
    result.push_str(&ad);
    *p_count += 1;
}

