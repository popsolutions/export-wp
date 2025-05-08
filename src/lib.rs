use lazy_static::lazy_static;
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
    // Regex para identificar múltiplas quebras de linha (agora em escopo global)
    static ref MULTI_NEWLINE: Regex = Regex::new(r"\n\s*\n").unwrap();
}

pub fn text_to_html_paragraphs(text: &str) -> String {
    let mut result = String::new();
    let mut last_end = 0;

    // Encontra todos os blocos especiais
    for cap in BLOCKS.captures_iter(text) {
        let m = cap.get(0).unwrap();
        
        // Processa o texto antes do bloco especial
        let before_block = &text[last_end..m.start()];
        if !before_block.trim().is_empty() {
            // Adiciona parágrafo antes do bloco especial
            result.push_str(&wrap_in_paragraphs(before_block));
        }
        
        // Adiciona o bloco especial
        result.push_str(m.as_str());
        
        last_end = m.end();
    }

    // Processa o texto restante após o último bloco especial
    let remaining = &text[last_end..];
    if !remaining.trim().is_empty() {
        result.push_str(&wrap_in_paragraphs(remaining));
    }
    result = result.replace("<<p", ". <p");
    result
}

// Função auxiliar para envolver texto em parágrafos
fn wrap_in_paragraphs(text: &str) -> String {
    let mut result = String::new();
    let paragraphs: Vec<&str> = MULTI_NEWLINE.split(text).collect();
    
    for (i, paragraph) in paragraphs.iter().enumerate() {
        let trimmed = paragraph.trim();
        if !trimmed.is_empty() {
            // Adiciona espaço entre parágrafos, exceto após o último
            let space = if i < paragraphs.len() - 1 { " " } else { "" };
            result.push_str(&format!("<p>{}</p>{}", trimmed, space));
        }
    }
    
    result
}
