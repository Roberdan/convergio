use super::{Template, TemplateParam, TemplateTask};

/// Workflow template: handle a GDPR DSAR (Art. 12(3)) with a hard 30-day SLA.
///
/// Notes:
/// - This template encodes the SLA and escalation checkpoints as task descriptions.
/// - Convergio does not (yet) schedule timers or auto-page on deadlines; treat the
///   "auto-escalation" items as required operational steps to wire into your org's
///   on-call / ticketing system.
pub const DSAR_GDPR_V1: Template = Template {
    name: "dsar-gdpr-v1",
    summary: "Process a GDPR DSAR end-to-end with an explicit 30-day SLA and escalation checkpoints.",
    description: Some(
        "Use for access/erasure/rectification/portability requests. \
         SLA: 30 days (GDPR Art. 12(3)) from `{{received_at_utc}}`. \
         Set internal checkpoints (T+7, T+14, T+21, T+25) and escalate before the hard stop.",
    ),
    objective: "Respond to DSAR {{request_id}} within 30 days (GDPR Art. 12(3)) with verified identity, complete processing for scope {{processing_scope}}, and a defensible audit trail.",
    parameters: &[
        TemplateParam {
            name: "request_id",
            help: "DSAR case ID (e.g. DSAR-2026-0007).",
        },
        TemplateParam {
            name: "received_at_utc",
            help: "Timestamp when request was received/acknowledged (UTC, ISO-8601).",
        },
        TemplateParam {
            name: "requester_contact",
            help: "How to reach the requester (email/portal handle).",
        },
        TemplateParam {
            name: "data_subject_identifier",
            help: "Stable identifier in systems (user_id / email / external_id).",
        },
        TemplateParam {
            name: "processing_scope",
            help: "Systems/products in scope (e.g. app, billing, support, logs).",
        },
        TemplateParam {
            name: "dpo_contact",
            help: "DPO / privacy contact for sign-off (name or role).",
        },
        TemplateParam {
            name: "response_language",
            help: "Language used for the response (e.g. en, it).",
        },
    ],
    title: "DSAR {{request_id}} ({{data_subject_identifier}})",
    tasks: &[
        TemplateTask {
            wave: 1,
            sequence: 1,
            title: "Open DSAR case {{request_id}} and start SLA clock (T0={{received_at_utc}})",
            description: Some(
                "Hard SLA: 30 days from T0 (GDPR Art. 12(3)).\n\
                 Checkpoints (set reminders / tickets): T+7 acknowledge + scope, T+14 data mapping, T+21 draft response, T+25 escalation to {{dpo_contact}} + leadership if risk.\n\
                 Auto-escalation requirement: wire reminders into your ticketing/on-call system for each checkpoint.",
            ),
            evidence_required: &["doc_link"],
        },
        TemplateTask {
            wave: 1,
            sequence: 2,
            title: "Verify identity / authority for {{requester_contact}}",
            description: Some(
                "Verify requester identity and authority to act for {{data_subject_identifier}}.\n\
                 If identity cannot be verified promptly, document the gap and request additional proof; do not disclose personal data before verification.",
            ),
            evidence_required: &["doc_link"],
        },
        TemplateTask {
            wave: 1,
            sequence: 3,
            title: "Triage DSAR type and scope for {{processing_scope}}",
            description: Some(
                "Confirm request type (access/erasure/rectification/portability/objection) and scope boundaries.\n\
                 Capture exclusions (e.g. third-party data) and any applicable legal basis for refusal/limitation.",
            ),
            evidence_required: &["doc_link"],
        },
        TemplateTask {
            wave: 2,
            sequence: 1,
            title: "Map data sources for {{data_subject_identifier}} across {{processing_scope}}",
            description: Some(
                "Produce a system-by-system checklist: where personal data may exist (prod DBs, backups, analytics, support tickets, logs).\n\
                 Assign owners per system and record expected retrieval/erasure method.",
            ),
            evidence_required: &["doc_link"],
        },
        TemplateTask {
            wave: 3,
            sequence: 1,
            title: "Collect data / execute changes and prepare response pack",
            description: Some(
                "For access/portability: export data in a structured, commonly used format.\n\
                 For rectification/erasure: execute changes and record resulting system state.\n\
                 Redact third-party personal data where required and document rationale.",
            ),
            evidence_required: &["doc_link"],
        },
        TemplateTask {
            wave: 4,
            sequence: 1,
            title: "Privacy/legal review + sign-off by {{dpo_contact}}",
            description: Some(
                "Confirm completeness, redactions, and legal basis for any denials/limitations.\n\
                 Validate that the response is ready to ship before T0+30 days.",
            ),
            evidence_required: &["doc_link"],
        },
        TemplateTask {
            wave: 5,
            sequence: 1,
            title: "Deliver response in {{response_language}} + archive evidence",
            description: Some(
                "Send response to {{requester_contact}} in {{response_language}}.\n\
                 Archive: response artifacts, timestamps, approvals, and the final system checklist for defensible audit.",
            ),
            evidence_required: &["doc_link"],
        },
    ],
};
