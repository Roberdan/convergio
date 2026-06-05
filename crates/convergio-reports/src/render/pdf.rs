use crate::error::{ReportError, Result};

pub(super) fn render_pdf_lopdf(
    body_text: &str,
    qr_png_bytes: &[u8],
    manifest_json: &str,
) -> Result<Vec<u8>> {
    use lopdf::content::{Content, Operation};
    use lopdf::{dictionary, Document, Object, Stream};

    fn text_ops(
        lines: &[String],
        font_size: i64,
        start_x: i64,
        start_y: i64,
        leading: i64,
    ) -> Vec<Operation> {
        let mut ops: Vec<Operation> = Vec::new();
        ops.push(Operation::new("BT", vec![]));
        ops.push(Operation::new("Tf", vec!["F1".into(), font_size.into()]));
        ops.push(Operation::new("Td", vec![start_x.into(), start_y.into()]));
        for (i, line) in lines.iter().enumerate() {
            if i > 0 {
                ops.push(Operation::new("Td", vec![0.into(), (-leading).into()]));
            }
            ops.push(Operation::new(
                "Tj",
                vec![Object::string_literal(line.as_str())],
            ));
        }
        ops.push(Operation::new("ET", vec![]));
        ops
    }

    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();

    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Courier",
    });
    let resources_id = doc.add_object(dictionary! {
        "Font" => dictionary! { "F1" => font_id },
    });

    let media_box = vec![0.into(), 0.into(), 595.into(), 842.into()];

    let body_lines: Vec<String> = body_text.lines().map(str::to_string).collect();
    let body_stream = Stream::new(
        dictionary! {},
        Content {
            operations: text_ops(&body_lines, 12, 50, 780, 14),
        }
        .encode()
        .map_err(|e| ReportError::Pdf(e.to_string()))?,
    );
    let body_content_id = doc.add_object(body_stream);
    let page1_id = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "Contents" => body_content_id,
        "MediaBox" => media_box.clone(),
    });

    let qr_stream = lopdf::xobject::image_from(qr_png_bytes.to_vec())
        .map_err(|e| ReportError::Pdf(e.to_string()))?;
    let qr_id = doc.add_object(qr_stream);

    let mut prov_ops: Vec<Operation> = Vec::new();
    prov_ops.extend(text_ops(
        &["Provenienza / Provenance".to_string()],
        16,
        50,
        780,
        18,
    ));
    prov_ops.push(Operation::new("q", vec![]));
    prov_ops.push(Operation::new(
        "cm",
        vec![
            200.into(),
            0.into(),
            0.into(),
            200.into(),
            50.into(),
            560.into(),
        ],
    ));
    prov_ops.push(Operation::new("Do", vec![Object::Name(b"Im1".to_vec())]));
    prov_ops.push(Operation::new("Q", vec![]));

    let manifest_lines: Vec<String> = manifest_json.lines().map(str::to_string).collect();
    prov_ops.extend(text_ops(&manifest_lines, 8, 50, 520, 10));

    let prov_stream = Stream::new(
        dictionary! {},
        Content {
            operations: prov_ops,
        }
        .encode()
        .map_err(|e| ReportError::Pdf(e.to_string()))?,
    );
    let prov_content_id = doc.add_object(prov_stream);
    let page2_id = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "Contents" => prov_content_id,
        "MediaBox" => media_box.clone(),
    });

    doc.add_xobject(page2_id, b"Im1", qr_id)
        .map_err(|e| ReportError::Pdf(e.to_string()))?;

    let pages = dictionary! {
        "Type" => "Pages",
        "Kids" => vec![page1_id.into(), page2_id.into()],
        "Count" => 2,
        "Resources" => resources_id,
        "MediaBox" => media_box,
    };
    doc.objects.insert(pages_id, Object::Dictionary(pages));

    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);

    let mut out: Vec<u8> = Vec::new();
    doc.save_to(&mut out)
        .map_err(|e| ReportError::Pdf(e.to_string()))?;
    Ok(out)
}
