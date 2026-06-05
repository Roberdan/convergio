use super::util::escape_html_text;

pub(super) fn append_provenance_html(
    body_html: &str,
    qr_data_uri: &str,
    manifest_json: &str,
    manifest_b64: &str,
    manifest_sha256: &str,
) -> String {
    let manifest_pretty = escape_html_text(manifest_json);
    format!(
        "<!doctype html>\n<html>\n<head>\n<meta charset=\"utf-8\" />\n<meta name=\"convergio-report-manifest-sha256\" content=\"{manifest_sha256}\" />\n</head>\n<body>\n{body_html}\n\n<section id=\"convergio-provenance\">\n<h2>Provenienza / Provenance</h2>\n<img alt=\"QR Provenienza / Provenance\" src=\"{qr_data_uri}\" />\n<pre id=\"convergio-report-manifest-pretty\">{manifest_pretty}</pre>\n<script type=\"application/json\" id=\"convergio-report-manifest\" data-encoding=\"base64\" data-base64=\"{manifest_b64}\"></script>\n</section>\n</body>\n</html>\n"
    )
}
