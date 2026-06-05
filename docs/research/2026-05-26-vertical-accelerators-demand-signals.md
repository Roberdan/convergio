# 2026-05-26 — Demand signals for two vertical accelerators (edu + research)

Tracks: Tbb408ef6-1eee-417e-8c8e-b702f995bfae

## Why this note exists

Convergio defines a **vertical accelerator** as a plan template + curated
capability blocks + domain gates (CONSTITUTION §4 / §17). The Workbench
accelerator stub must not ship without proof that **at least two verticals**
have real demand.

## TL;DR (high-signal)

- `convergio-edu` demand signal is *not* “teachers like AI”; it’s that
  **workload + privacy + accessibility** are structural constraints, so a
  “leash” with gates is credibly valuable.
- `convergio-research` demand signal is that **funders require DMS/DMP
  compliance** and researchers report **large administrative burden**; a
  vertical accelerator can package the compliance loop as gated work.

## Definition (repo vocabulary)

- “A **vertical accelerator** (e.g. `convergio-edu`, `convergio-research`, …)
  is a plan template plus capability blocks plus domain gates.”
  (CONSTITUTION §17) [R1]

## Vertical 1 — `convergio-edu` (education)

### Buyer / operator shape

- Operator is typically a district / school / NGO / founder building a
  dyslexia-friendly and multilingual education app (ROADMAP Wave 3
  explicitly frames `convergio-edu` around dyslexia-friendly + EN+IT). [R2]

### Demand signals (concrete)

1) **Administrative workload is explicitly targeted for reduction (5h/week).**

- NSW Government’s teacher admin-task audit states it was commissioned
  “to deliver a reduction of **5 hours of administrative work per week**”.
  That is an explicit policy-level admission that admin overhead is big
  enough to measure-and-reduce. [E1]

2) **Education AI policy explicitly calls out privacy + wrong outputs.**

- US Dept. of Education’s 2023 report notes educators “are also aware of
  new risks”, including “data privacy and security risks” and that AI “can
  automatically produce output that is inappropriate or wrong.” [E2]

3) **A large dyslexia-affected population exists (accessibility-first as product constraint).**

- International Dyslexia Association: “perhaps as many as **15–20% of the
  population as a whole** have some of the symptoms of dyslexia …”. [E3]

4) **FERPA constrains vendor behavior when student PII is involved.**

- US Dept. of Education PTAC guidance for third-party service providers
  explains FERPA’s “school official exception” constraints (direct control,
  authorized purpose only, no re-disclosure) and notes that indirect
  identifiers/metadata may still count as PII. [E4]

### What an “education accelerator” must package (implications for Workbench)

Workbench must let an accelerator author **declare constraints first**, not
as an afterthought:

- Template params (minimum viable):
  - `primary_language`, `secondary_language` (CONSTITUTION P5 pressure)
  - `accessibility_profile` (ROADMAP defaults to dyslexia-friendly) [R2]
  - `student_pii_boundary` (domain gate config)
- Evidence kinds likely needed in the first demo:
  - `ui.fluent_bundle` (prove i18n coverage)
  - `a11y.report` / `a11y.screenshot` (prove accessible UI)
  - `privacy.data_flow` (prove PII handling boundaries)
- Domain gates (education-specific):
  - “student PII boundary” gate (FERPA/GDPR-adjacent posture)
  - multilingual gate (refuse hardcoded strings)
  - a11y gate with severity threshold

## Vertical 2 — `convergio-research` (research / labs / grants)

### Buyer / operator shape

- Operator is a PI / lab manager / research operations lead who must ship
  compliant artifacts (plans, reports, datasets, metadata) under funder rules.

### Demand signals (concrete)

1) **Researchers report high administrative burden (42%).**

- FDP faculty burden survey paper (Res Manag Rev, 2009): “42% of the time
  spent by an average PI on a federally funded research project was …
  expended on administrative tasks … rather than on research.” [R3]

2) **NIH DMS policy makes data management & sharing plans a required loop.**

- NIH notice NOT-OD-21-013 states the final policy “establishes the
  requirements of submission of Data Management and Sharing Plans … and
  compliance with … approved Plans”, effective Jan 25, 2023. [R4]

3) **NSF expectations for data sharing are explicit.**

- NSF: investigators are “expected to share … the primary data, samples,
  physical collections and other supporting materials … within a reasonable
  time”. NSF proposals require a data management and sharing plan. [R5]

### What a “research accelerator” must package (implications for Workbench)

Workbench should support a “compliance-first” accelerator authoring loop:

- Template params (minimum viable):
  - `funder` (e.g., NIH/NSF/EU) → selects policy checklist
  - `data_sensitivity` (human subjects? genomic? etc.)
  - `sharing_scope` (open / controlled / embargo)
- Evidence kinds likely needed:
  - `dms_plan` (machine-readable + rendered)
  - `repository_selection_rationale`
  - `data_dictionary` / `metadata_schema`
  - `risk_assessment` (privacy/security posture)
- Domain gates:
  - “DMS plan present + complete” gate
  - “sensitive data handling declared” gate
  - “sharing deadline / deliverable checklist” gate

## Cross-vertical requirements for the Workbench accelerator stub

Both verticals share the same non-negotiables:

- **Configurable domain gates** must be first-class (not hardcoded).
- **Evidence schema composition** must be expressible in templates.
- The stub must support “policy/procurement reality”: privacy boundaries,
  auditability, and an explicit human-in-the-loop review seam.

## Next steps (actionable)

1) Update / confirm the Workbench stub’s acceptance criteria: “can express
   template params + evidence kinds + domain gates for edu + research.”
2) When implementing, link this note in the PR body and ensure the first
   template(s) can represent:
   - `education-accelerator-v1` (ROADMAP Wave 3)
   - `research-accelerator-v1` (ROADMAP second-vertical requirement)

---

## Sources

### Repo sources

- [R1] [`CONSTITUTION.md`](../../CONSTITUTION.md) (vertical accelerator definition)
- [R2] [`ROADMAP.md`](../../ROADMAP.md) (Wave 3 and second-vertical requirement)

### External sources

- [E1] NSW Dept. of Education — *Audit of Teachers’ Administrative Tasks (Summary)*
  https://education.nsw.gov.au/content/dam/main-education/about-us/strategies-and-reports/workload-reduction/Audit_of_teachers_administrative_tasks_summary.pdf
- [E2] U.S. Dept. of Education (Office of Educational Technology) — *Artificial Intelligence and the Future of Teaching and Learning* (May 2023)
  https://www2.ed.gov/documents/ai-report/ai-report.pdf
- [E3] International Dyslexia Association — *Dyslexia Basics* (“15–20% … symptoms”)
  https://dyslexiaida.org/dyslexia-basics/
- [E4] U.S. Dept. of Education PTAC — *Responsibilities of Third-Party Service Providers under FERPA* (Aug 2015)
  https://studentprivacy.ed.gov/sites/default/files/resource_document/file/Vendor%20FAQ.pdf

- [R3] Federal Demonstration Partnership faculty burden survey (PMC mirror)
  https://www.ncbi.nlm.nih.gov/pmc/articles/PMC2887040/
- [R4] NIH — NOT-OD-21-013 Final NIH Policy for Data Management and Sharing
  https://grants.nih.gov/grants/guide/notice-files/NOT-OD-21-013.html
- [R5] NSF — Data management and sharing plan overview
  https://www.nsf.gov/bfa/dias/policy/dmp.jsp
