use crate::error::{ReportError, Result};

pub(super) fn render_docx(
    body_text: &str,
    qr_png_bytes: &[u8],
    manifest_json: &str,
) -> Result<Vec<u8>> {
    use docx_rs::{Docx, Paragraph, Pic, Run};

    let mut doc = Docx::new();

    for line in body_text.lines() {
        doc = doc.add_paragraph(Paragraph::new().add_run(Run::new().add_text(line)));
    }

    doc = doc.add_paragraph(Paragraph::new().add_run(Run::new().add_text("")));
    doc = doc
        .add_paragraph(Paragraph::new().add_run(Run::new().add_text("Provenienza / Provenance")));

    let pic = Pic::new(qr_png_bytes).size(140 * 9525, 140 * 9525);
    doc = doc.add_paragraph(Paragraph::new().add_run(Run::new().add_image(pic)));

    for line in manifest_json.lines() {
        doc = doc.add_paragraph(Paragraph::new().add_run(Run::new().add_text(line)));
    }

    let mut out = std::io::Cursor::new(Vec::new());
    doc.build()
        .pack(&mut out)
        .map_err(|e| ReportError::Docx(e.to_string()))?;
    Ok(out.into_inner())
}
