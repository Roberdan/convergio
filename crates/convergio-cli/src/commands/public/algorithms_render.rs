use super::algorithms_schema::{AlgorithmEntry, BilingualText, Reference, RiskClass};

pub(super) fn render_index_html(
    tenant: &str,
    generated_at: Option<&str>,
    algos: &[AlgorithmEntry],
) -> String {
    let mut body = String::new();
    body.push_str("<h1>Algorithm Register / Registro degli algoritmi</h1>\n");
    body.push_str(&format!("<p><strong>Tenant:</strong> {} </p>\n", h(tenant)));
    if let Some(ts) = generated_at {
        body.push_str(&format!("<p><strong>Generated:</strong> {} </p>\n", h(ts)));
    }

    if algos.is_empty() {
        body.push_str("<p>No algorithms / Nessun algoritmo.</p>\n");
    } else {
        body.push_str("<ul>\n");
        for a in algos {
            body.push_str(&format!(
                "  <li><a href=\"{slug}/\">{title_en}</a> — <span class=\"muted\">{action}</span><br/><span class=\"muted\">{title_it}</span></li>\n",
                slug = h(&a.slug),
                title_en = h(&a.title.en),
                title_it = h(&a.title.it),
                action = h(&a.action),
            ));
        }
        body.push_str("</ul>\n");
    }

    page("Algorithm Register", &body)
}

pub(super) fn render_algorithm_html(
    tenant: &str,
    generated_at: Option<&str>,
    a: &AlgorithmEntry,
) -> String {
    let mut body = String::new();

    body.push_str("<p><a href=\"../\">&larr; Back / Indietro</a></p>\n");

    body.push_str(&format!(
        "<h1>{} <span class=\"muted\">({})</span></h1>\n",
        h(&a.title.en),
        h(&a.action)
    ));
    body.push_str(&format!("<p class=\"muted\">{}</p>\n", h(&a.title.it)));

    body.push_str("<dl class=\"grid\">\n");
    body.push_str(&dl_bilingual("Purpose / Scopo", &a.purpose));
    body.push_str(&dl_bilingual(
        "Lawful basis / Base giuridica",
        &a.lawful_basis,
    ));

    body.push_str("<dt>Data categories / Categorie di dati</dt><dd>");
    if a.data_categories.is_empty() {
        body.push_str(none());
    } else {
        body.push_str("<ul>");
        for c in &a.data_categories {
            body.push_str(&format!(
                "<li><span class=\"lang\">EN</span> {}<br/><span class=\"lang\">IT</span> {}</li>",
                h(&c.en),
                h(&c.it)
            ));
        }
        body.push_str("</ul>");
    }
    body.push_str("</dd>\n");

    body.push_str("<dt>Model / Modello</dt><dd>");
    body.push_str(&format!("<div><strong>{}</strong></div>", h(&a.model.name)));
    if let Some(v) = a.model.version.as_deref() {
        body.push_str(&format!("<div class=\"muted\">Version: {}</div>", h(v)));
    }
    if let Some(p) = a.model.provider.as_deref() {
        body.push_str(&format!("<div class=\"muted\">Provider: {}</div>", h(p)));
    }
    body.push_str("</dd>\n");

    body.push_str(&dl_line("Region / Regione", &a.region));
    body.push_str(&dl_bilingual("Oversight / Supervisione", &a.oversight));
    body.push_str(&dl_line(
        "Risk class / Classe di rischio",
        match a.risk_class {
            RiskClass::Low => "low",
            RiskClass::Medium => "medium",
            RiskClass::High => "high",
            RiskClass::Critical => "critical",
        },
    ));

    body.push_str("<dt>Eval scorecard / Scorecard di valutazione</dt><dd>");
    if let Some(sc) = &a.eval_scorecard {
        body.push_str(&format!(
            "<div><a href=\"{url}\">{en}</a></div><div class=\"muted\">{it}</div>",
            url = h(&sc.url),
            en = h(&sc.title.en),
            it = h(&sc.title.it)
        ));
    } else {
        body.push_str(none());
    }
    body.push_str("</dd>\n");

    body.push_str("<dt>DPIA refs / Riferimenti DPIA</dt><dd>");
    if a.dpia_refs.is_empty() {
        body.push_str(none());
    } else {
        body.push_str(&refs_list(&a.dpia_refs));
    }
    body.push_str("</dd>\n");

    body.push_str("<dt>Ethics refs / Riferimenti etici</dt><dd>");
    if a.ethics_refs.is_empty() {
        body.push_str(none());
    } else {
        body.push_str(&refs_list(&a.ethics_refs));
    }
    body.push_str("</dd>\n");

    body.push_str(&dl_bilingual("Limitations / Limitazioni", &a.limitations));

    body.push_str("<dt>Appeal contact / Contatto per ricorso</dt><dd>");
    let mut any = false;
    if let Some(email) = a.appeal_contact.email.as_deref() {
        any = true;
        body.push_str(&format!(
            "<div>Email: <a href=\"mailto:{e}\">{e}</a></div>",
            e = h(email)
        ));
    }
    if let Some(url) = a.appeal_contact.url.as_deref() {
        any = true;
        body.push_str(&format!(
            "<div>URL: <a href=\"{u}\">{u}</a></div>",
            u = h(url)
        ));
    }
    if let Some(notes) = &a.appeal_contact.notes {
        any = true;
        body.push_str(&format!(
            "<div class=\"bilingual\"><div><span class=\"lang\">EN</span> {}</div><div><span class=\"lang\">IT</span> {}</div></div>",
            h(&notes.en),
            h(&notes.it)
        ));
    }
    if !any {
        body.push_str(none());
    }
    body.push_str("</dd>\n");

    body.push_str("</dl>\n");

    body.push_str(&format!(
        "<hr/><p class=\"muted\"><strong>Tenant:</strong> {} &middot; <strong>Slug:</strong> {}",
        h(tenant),
        h(&a.slug)
    ));
    if let Some(ts) = generated_at {
        body.push_str(&format!(" &middot; <strong>Generated:</strong> {}", h(ts)));
    }
    body.push_str("</p>\n");

    page(&format!("Algorithm: {}", a.slug), &body)
}

fn refs_list(refs: &[Reference]) -> String {
    let mut out = String::from("<ul>");
    for r in refs {
        out.push_str(&format!(
            "<li><a href=\"{url}\">{en}</a><br/><span class=\"muted\">{it}</span></li>",
            url = h(&r.url),
            en = h(&r.title.en),
            it = h(&r.title.it)
        ));
    }
    out.push_str("</ul>");
    out
}

fn dl_bilingual(label: &str, text: &BilingualText) -> String {
    format!(
        "<dt>{}</dt><dd><div class=\"bilingual\"><div><span class=\"lang\">EN</span> {}</div><div><span class=\"lang\">IT</span> {}</div></div></dd>\n",
        h(label),
        h(&text.en),
        h(&text.it)
    )
}

fn dl_line(label: &str, value: &str) -> String {
    format!("<dt>{}</dt><dd>{}</dd>\n", h(label), h(value))
}

fn page(title: &str, body: &str) -> String {
    format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\"/>\n<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"/>\n<title>{}</title>\n<style>\n{}\n</style>\n</head>\n<body>\n<div class=\"container\">\n{}\n</div>\n</body>\n</html>\n",
        h(title),
        css(),
        body
    )
}

fn none() -> &'static str {
    "<span class=\"muted\">(none / nessuno)</span>"
}

fn css() -> &'static str {
    r#"
:root { color-scheme: light dark; }
body { font-family: system-ui, -apple-system, Segoe UI, Roboto, sans-serif; line-height: 1.4; margin: 0; padding: 0; }
.container { max-width: 900px; margin: 0 auto; padding: 24px; }
a { color: inherit; }
.muted { opacity: 0.75; }
.grid { display: grid; grid-template-columns: 1fr 2fr; gap: 12px 16px; }
.grid dt { font-weight: 700; }
.grid dd { margin: 0; }
.bilingual { display: grid; grid-template-columns: 1fr; gap: 6px; }
.lang { font-size: 0.75rem; font-weight: 700; padding: 2px 6px; border-radius: 6px; border: 1px solid currentColor; margin-right: 6px; display: inline-block; }
hr { margin: 24px 0; opacity: 0.4; }
@media (min-width: 720px) {
  .bilingual { grid-template-columns: 1fr 1fr; }
}
"#
}

fn h(s: &str) -> String {
    // Minimal escaping to prevent accidental HTML injection.
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}
