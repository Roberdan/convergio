use super::{Template, TemplateParam, TemplateTask};

/// Workflow template: handle a suspected personal-data breach with GDPR deadlines.
///
/// Hard SLAs encoded in task descriptions:
/// - Art. 33: notify supervisory authority within 72 hours of becoming aware.
/// - Art. 34: notify affected data subjects without undue delay; this template uses
///   a 60-day hard internal deadline (per task contract) to force escalation.
///
/// Auto-escalation:
/// Convergio does not (yet) run scheduled timers; this template includes explicit
/// tasks to wire escalation into on-call/ticketing immediately after T0.
pub const INCIDENT_RESPONSE_GDPR_V1: Template = Template {
    name: "incident-response-gdpr-v1",
    summary: "Incident response workflow for a suspected GDPR personal-data breach (72h/60d deadlines + escalation).",
    description: Some(
        "Use when you suspect unauthorized access/exfiltration involving personal data. \
         Start the clock at `{{aware_at_utc}}` (T0). \
         SLA: 72h to supervisory authority (Art. 33). Internal hard stop: 60d for data subject comms (Art. 34) when required.",
    ),
    objective: "Contain, assess, and remediate incident {{incident_id}} affecting {{system}} while meeting GDPR notification obligations (Art. 33 within 72h from T0={{aware_at_utc}}; Art. 34 communications by internal deadline) with documented escalation.",
    parameters: &[
        TemplateParam {
            name: "incident_id",
            help: "Incident identifier (e.g. INC-2026-0412).",
        },
        TemplateParam {
            name: "aware_at_utc",
            help: "Timestamp when you became aware (UTC, ISO-8601).",
        },
        TemplateParam {
            name: "system",
            help: "Primary system/service impacted (e.g. billing-api).",
        },
        TemplateParam {
            name: "suspected_data",
            help: "Suspected personal data categories (e.g. emails, IPs, health data).",
        },
        TemplateParam {
            name: "security_contact",
            help: "Security lead / incident commander.",
        },
        TemplateParam {
            name: "dpo_contact",
            help: "DPO / privacy lead.",
        },
        TemplateParam {
            name: "supervisory_authority",
            help: "Supervisory authority (e.g. Garante per la protezione dei dati personali).",
        },
        TemplateParam {
            name: "comms_contact",
            help: "External communications / legal comms owner.",
        },
    ],
    title: "Incident {{incident_id}} ({{system}})",
    tasks: &[
        TemplateTask {
            wave: 1,
            sequence: 1,
            title: "Declare incident {{incident_id}} + start breach clock (T0={{aware_at_utc}})",
            description: Some(
                "Assign roles: incident commander={{security_contact}}, privacy={{dpo_contact}}, comms={{comms_contact}}.\n\
                 Escalation checkpoints (set reminders/tickets immediately): T+24h, T+48h, T+60h (72h hard stop).\n\
                 Auto-escalation requirement: wire these checkpoints into on-call/ticketing so missed milestones page the incident commander.",
            ),
            evidence_required: &["doc_link"],
        },
        TemplateTask {
            wave: 1,
            sequence: 2,
            title: "Containment: isolate {{system}} and stop further exposure",
            description: Some(
                "Actions may include: disable compromised accounts/keys, rotate secrets, block egress, take affected components out of rotation.\n\
                 Preserve service availability impact notes for comms.",
            ),
            evidence_required: &["doc_link"],
        },
        TemplateTask {
            wave: 1,
            sequence: 3,
            title: "Preserve evidence + begin forensics (logs, snapshots, chain-of-custody)",
            description: Some(
                "Freeze relevant logs and artifacts; document chain-of-custody.\n\
                 Avoid destroying evidence during containment. Capture initial IOCs and timeline.",
            ),
            evidence_required: &["doc_link"],
        },
        TemplateTask {
            wave: 2,
            sequence: 1,
            title: "Assess whether personal data was affected ({{suspected_data}}) and if Art. 33 reporting is required",
            description: Some(
                "Determine likelihood/severity of risk to rights and freedoms.\n\
                 Document: categories/approx counts, affected regions, duration, attack vector, and mitigations already applied.",
            ),
            evidence_required: &["doc_link"],
        },
        TemplateTask {
            wave: 2,
            sequence: 2,
            title: "Draft Art. 33 notification pack for {{supervisory_authority}}",
            description: Some(
                "Include (minimum): nature of breach, categories/approx number of data subjects + records, likely consequences, measures taken/proposed, and DPO contact={{dpo_contact}}.\n\
                 If information is incomplete, prepare phased notification and commit to follow-up.",
            ),
            evidence_required: &["doc_link"],
        },
        TemplateTask {
            wave: 3,
            sequence: 1,
            title: "Submit Art. 33 notification within 72h of T0 (or document delay reasons)",
            description: Some(
                "Hard stop: 72 hours from {{aware_at_utc}}.\n\
                 If late, document reasons for delay and what was known when. Record submission timestamp and reference number.",
            ),
            evidence_required: &["doc_link"],
        },
        TemplateTask {
            wave: 3,
            sequence: 2,
            title: "Decide whether Art. 34 data subject notification is required and draft comms",
            description: Some(
                "Assess 'high risk' threshold and exceptions (e.g. effective encryption).\n\
                 Draft clear guidance for data subjects: what happened, what data, what you did, recommended actions, support channels.",
            ),
            evidence_required: &["doc_link"],
        },
        TemplateTask {
            wave: 4,
            sequence: 1,
            title: "Notify affected data subjects when required (internal hard deadline: 60 days)",
            description: Some(
                "Internal contract: complete notifications within 60 days of T0 unless a documented exception applies.\n\
                 Escalate to exec + {{dpo_contact}} if comms are not ready by T0+30d.",
            ),
            evidence_required: &["doc_link"],
        },
        TemplateTask {
            wave: 5,
            sequence: 1,
            title: "Remediation plan + control improvements (prevent recurrence + improve escalation)",
            description: Some(
                "Patch root cause, rotate credentials, add monitoring/alerting, and update runbooks.\n\
                 Capture follow-up tasks for longer-term fixes and validate controls.",
            ),
            evidence_required: &["doc_link"],
        },
    ],
};
