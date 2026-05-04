//! Output renderers for `cvg pr stack`.
//!
//! Split out of `pr.rs` to keep both files under the 300-line cap.
//! Three modes per the CLI's global `--output` flag:
//!
//! - `human` (default): a one-line summary per PR plus a
//!   "Suggested merge order" arrow chain, all localised via
//!   [`convergio_i18n::Bundle`].
//! - `json`: structured output for piping into other tools.
//! - `plain`: bare PR numbers, in the suggested merge order, one
//!   per line — designed for shell pipelines like
//!   `for pr in $(cvg pr stack --output plain); do gh pr view $pr; done`.

use super::pr::{AnalysedPr, ManifestStatus};
use super::OutputMode;
use anyhow::Result;
use convergio_i18n::Bundle;

pub(crate) fn render(
    bundle: &Bundle,
    output: OutputMode,
    prs: &[AnalysedPr],
    order: &[i64],
) -> Result<()> {
    match output {
        OutputMode::Plain => {
            for n in order {
                println!("{n}");
            }
        }
        OutputMode::Json => {
            let value = serde_json::json!({
                "prs": prs.iter().map(|p| serde_json::json!({
                    "number": p.number,
                    "title": p.title,
                    "files": p.files,
                    "depends_on": p.depends_on,
                    "manifest_status": manifest_status_str(p.manifest_status),
                })).collect::<Vec<_>>(),
                "suggested_order": order,
            });
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        OutputMode::Human => render_human(bundle, prs, order),
    }
    Ok(())
}

fn manifest_status_str(s: ManifestStatus) -> &'static str {
    match s {
        ManifestStatus::Match => "match",
        ManifestStatus::Missing => "missing",
        ManifestStatus::Mismatch => "mismatch",
    }
}

fn render_human(bundle: &Bundle, prs: &[AnalysedPr], order: &[i64]) {
    if prs.is_empty() {
        println!("{}", bundle.t("pr-stack-empty", &[]));
        return;
    }
    let count = prs.len().to_string();
    println!("{}", bundle.t("pr-stack-header", &[("count", &count)]));
    for p in prs {
        let manifest = manifest_label(bundle, p);
        let deps = if p.depends_on.is_empty() {
            String::new()
        } else {
            format!(
                " depends-on=[{}]",
                p.depends_on
                    .iter()
                    .map(|n| format!("#{n}"))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        };
        let conflicts = compute_conflicts(p, prs);
        let conflicts_str = if conflicts.is_empty() {
            String::new()
        } else {
            format!(
                " conflicts-with=[{}]",
                conflicts
                    .iter()
                    .map(|n| format!("#{n}"))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        };
        println!(
            "  #{:<4} {} ({}){}{}",
            p.number,
            truncate(&p.title, 60),
            manifest,
            deps,
            conflicts_str
        );
    }
    println!();
    print!("{} ", bundle.t("pr-stack-suggested-order", &[]));
    println!(
        "{}",
        order
            .iter()
            .map(|n| format!("#{n}"))
            .collect::<Vec<_>>()
            .join(" -> ")
    );
}

fn manifest_label(bundle: &Bundle, p: &AnalysedPr) -> String {
    match p.manifest_status {
        ManifestStatus::Missing => bundle.t("pr-stack-no-manifest", &[]),
        ManifestStatus::Mismatch => bundle.t("pr-stack-manifest-mismatch", &[]),
        ManifestStatus::Match => {
            let count = p.files.len().to_string();
            bundle.t("pr-stack-files-summary", &[("count", &count)])
        }
    }
}

fn compute_conflicts(target: &AnalysedPr, all: &[AnalysedPr]) -> Vec<i64> {
    let mut out = Vec::new();
    for p in all {
        if p.number == target.number {
            continue;
        }
        if target.files.intersection(&p.files).next().is_some() {
            out.push(p.number);
        }
    }
    out
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let mut buf: String = s.chars().take(n.saturating_sub(1)).collect();
        buf.push('…');
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use convergio_i18n::Locale;
    use std::collections::BTreeSet;

    fn pr(number: i64, files: &[&str], status: ManifestStatus) -> AnalysedPr {
        AnalysedPr {
            number,
            title: format!("pr-{number}"),
            files: files.iter().map(|s| s.to_string()).collect(),
            depends_on: BTreeSet::new(),
            manifest_status: status,
        }
    }

    #[test]
    fn manifest_label_uses_localised_string_for_missing_en() {
        let bundle = Bundle::new(Locale::En).unwrap();
        let p = pr(1, &[], ManifestStatus::Missing);
        let label = manifest_label(&bundle, &p);
        assert!(
            label.to_lowercase().contains("manifest"),
            "expected English manifest label, got: {label}"
        );
    }

    #[test]
    fn manifest_label_uses_localised_string_for_mismatch_it() {
        let bundle = Bundle::new(Locale::It).unwrap();
        let p = pr(2, &["a", "b"], ManifestStatus::Mismatch);
        let label = manifest_label(&bundle, &p);
        assert!(
            label.contains("manifest") && label.contains("diff"),
            "expected Italian mismatch label, got: {label}"
        );
    }

    #[test]
    fn manifest_label_pluralises_files_summary() {
        let bundle = Bundle::new(Locale::En).unwrap();
        let one = manifest_label(&bundle, &pr(3, &["a"], ManifestStatus::Match));
        let many = manifest_label(&bundle, &pr(4, &["a", "b", "c"], ManifestStatus::Match));
        assert!(one.contains("one") || one.contains("1"), "{one}");
        assert!(many.contains('3'), "{many}");
    }

    #[test]
    fn json_status_codes_are_machine_stable() {
        assert_eq!(manifest_status_str(ManifestStatus::Match), "match");
        assert_eq!(manifest_status_str(ManifestStatus::Missing), "missing");
        assert_eq!(manifest_status_str(ManifestStatus::Mismatch), "mismatch");
    }
}
