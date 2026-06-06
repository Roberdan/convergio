use crate::error::{ReportError, Result};
use jsonschema::Draft;
use minijinja::Environment;
use sha2::{Digest, Sha256};

pub(super) fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

pub(super) fn escape_html_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            _ => out.push(ch),
        }
    }
    out
}

pub(super) fn render_jinja(template: &str, params: &serde_json::Value) -> Result<String> {
    let mut env = Environment::new();
    env.add_template("t", template)
        .map_err(|e| ReportError::Template(e.to_string()))?;

    let t = env
        .get_template("t")
        .map_err(|e| ReportError::Template(e.to_string()))?;

    // If params is an object, expose keys at the top-level for ergonomic templates.
    if let serde_json::Value::Object(map) = params {
        let mut merged = map.clone();
        // Also provide the full params map at `params`.
        merged.insert("params".into(), params.clone());
        t.render(minijinja::value::Value::from_serialize(merged))
            .map_err(|e| ReportError::Template(e.to_string()))
    } else {
        t.render(minijinja::context! { params => params })
            .map_err(|e| ReportError::Template(e.to_string()))
    }
}

pub(super) fn validate_params(
    schema: &serde_json::Value,
    params: &serde_json::Value,
) -> Result<()> {
    let compiled = jsonschema::options()
        .with_draft(Draft::Draft7)
        .build(schema)
        .map_err(|e| ReportError::ParamValidation(e.to_string()))?;

    if let Err(errors) = compiled.validate(params) {
        // Take the first error for a stable, compact surface.
        let msg = errors
            .into_iter()
            .next()
            .map(|e| e.to_string())
            .unwrap_or_else(|| "unknown schema validation error".to_string());
        return Err(ReportError::ParamValidation(msg));
    }

    Ok(())
}
