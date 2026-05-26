use super::{Template, TemplateParam, TemplateTask};

/// First-party template: a generic vertical-accelerator scaffold.
pub const VERTICAL_ACCELERATOR_V1: Template = Template {
    name: "vertical-accelerator-v1",
    summary: "Scaffold a vertical-accelerator plan for a given domain.",
    description: Some(
        "Bootstraps a vertical accelerator for `{{domain}}` targeting `{{target_audience}}`. \
         Produces tasks for problem discovery, MVP scoping, prototype build, validation, and launch.",
    ),
    objective: "Ship a working {{domain}} vertical accelerator for {{target_audience}} that proves the loop end-to-end.",
    parameters: &[
        TemplateParam {
            name: "domain",
            help: "Vertical domain (e.g. education, healthcare).",
        },
        TemplateParam {
            name: "primary_language",
            help: "Primary user-facing language (BCP-47, e.g. en, it).",
        },
        TemplateParam {
            name: "secondary_language",
            help: "Secondary user-facing language (BCP-47).",
        },
        TemplateParam {
            name: "target_audience",
            help: "Who the accelerator is for (e.g. K-12 teachers).",
        },
    ],
    title: "{{domain}} vertical accelerator",
    tasks: &[
        TemplateTask {
            wave: 1,
            sequence: 1,
            title: "Discover top-3 pains for {{target_audience}} in {{domain}}",
            description: Some("Interview proxies / existing materials. Output: ranked pains doc."),
            evidence_required: &["doc_link"],
        },
        TemplateTask {
            wave: 2,
            sequence: 1,
            title: "Scope MVP slice for {{domain}}",
            description: Some("Pick the smallest pain that proves the loop end-to-end."),
            evidence_required: &["doc_link"],
        },
        TemplateTask {
            wave: 3,
            sequence: 1,
            title: "Build prototype in {{primary_language}}",
            description: Some(
                "Localised UI strings keyed by Fluent; {{secondary_language}} bundle stub.",
            ),
            evidence_required: &["code", "test_output"],
        },
        TemplateTask {
            wave: 4,
            sequence: 1,
            title: "Validate with {{target_audience}}",
            description: Some("Run 3+ sessions. Capture verbatim quotes."),
            evidence_required: &["doc_link"],
        },
        TemplateTask {
            wave: 5,
            sequence: 1,
            title: "Launch checklist for {{domain}} accelerator",
            description: Some("A11y, i18n, security review. Sign-off doc."),
            evidence_required: &["doc_link", "code"],
        },
    ],
};
