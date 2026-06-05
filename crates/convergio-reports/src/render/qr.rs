use crate::error::{ReportError, Result};
use qrcode::QrCode;

pub(super) fn qr_png(payload: &str) -> Result<Vec<u8>> {
    let code = QrCode::new(payload.as_bytes()).map_err(|e| ReportError::Qr(e.to_string()))?;
    let image = code
        .render::<image::Luma<u8>>()
        .min_dimensions(256, 256)
        .build();

    use image::ImageEncoder;

    let mut out = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut out);
    encoder
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            image::ExtendedColorType::L8,
        )
        .map_err(|e| ReportError::Qr(e.to_string()))?;
    Ok(out)
}
