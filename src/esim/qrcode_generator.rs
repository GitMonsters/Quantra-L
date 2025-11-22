use anyhow::Result;
use qrcode::QrCode;
use image::Luma;

pub fn generate_qr_code(data: &str) -> Result<Vec<u8>> {
    let code = QrCode::new(data.as_bytes())?;

    let image = code.render::<Luma<u8>>()
        .min_dimensions(512, 512)
        .build();

    let mut buffer = Vec::new();
    {
        use image::ImageEncoder;
        let encoder = image::codecs::png::PngEncoder::new(&mut buffer);
        let (width, height) = image.dimensions();
        encoder.write_image(image.as_raw(), width, height, image::ExtendedColorType::L8)?;
    }

    Ok(buffer)
}

pub fn generate_qr_code_svg(data: &str) -> Result<String> {
    let code = QrCode::new(data.as_bytes())?;

    let svg = code.render::<char>()
        .min_dimensions(512, 512)
        .build();

    Ok(svg)
}
