use killer::process_image_url;

#[test]
fn test_process_image_url_dataorg() {
    let image_to_process = String::from("https;://data.org/pt/wp-content/uploads/2022/02/1032236189_0_0_3026_2048_600x0_80_0_1_c249d36098212f019e38be7a6bed98be.jpg");
    let image_processed = String::from("/images/2022/02/1032236189_0_0_3026_2048_600x0_80_0_1_c249d36098212f019e38be7a6bed98be.jpg");
    let result = process_image_url(&image_to_process);
    assert_eq!(result, image_processed);
}

#[test]
fn test_process_image_url_optionorg() {
    let image_to_process = "https://www.option.org/wp-content/uploads/2022/05/WhatsApp-Image-2022-05-16-at-17.08.43-1.jpeg";
    let image_processed = "/images/2022/05/WhatsApp-Image-2022-05-16-at-17.08.43-1.jpeg";
    let result = process_image_url(image_to_process);
    assert_eq!(result, image_processed);
}
