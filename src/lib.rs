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
