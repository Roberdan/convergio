# EDCI / ELM accreditation self-assessment template (v1)

> **Purpose:** a pragmatic self-assessment template for organisations that
> publish **accreditation metadata** (ELM Accreditation AP) and/or issue
> **accreditation decisions** as EDCI credentials.
>
> **Status:** template only (Convergio does not publish these automatically).
>
> **Companion:** see `edci-accreditation-self-assessment.json` for a
> machine-readable checklist response shape.

## Alignment references

- ELM Browser (official docs): https://europa.eu/europass/elm-browser/index.html
- ELM namespace: http://data.europa.eu/snb/model/elm/
- EDC application profile (includes `elm:Accreditation` constraints):
  https://europa.eu/europass/elm-browser/homepage/3-2-0/edc-generic-no-cv_en.html

## Scope

This template covers two layers:

1. **Governance / QA readiness** (process maturity): policies, evidence, auditability.
2. **Data readiness** (ELM/EDCI metadata): do we have the fields needed to publish
   accreditation metadata and link it to qualifications/organisations.

---

## A) Basic identification

- Institution / Istituzione: `{{institution_name}}`
- Unit / Unità: `{{unit_name}}`
- Country / Paese: `{{country}}`
- Assessment date / Data autovalutazione (YYYY-MM-DD): `{{assessment_date}}`
- Responsible person / Responsabile: `{{responsible_person}}`

---

## B) Governance & quality assurance checklist (ESG-inspired)

> Provide: (1) short answer, (2) evidence links, (3) gaps/actions.

### B1. Quality assurance policy

- Question (EN): Do we have a published quality assurance policy covering programmes and awards?
- Domanda (IT): Esiste una politica di assicurazione della qualità pubblicata che copre programmi e titoli?
- Answer / Risposta: `{{b1_answer}}`
- Evidence / Evidenze: `{{b1_evidence}}`
- Gaps / Azioni: `{{b1_gaps}}`

### B2. Programme design & approval

- Answer / Risposta: `{{b2_answer}}`
- Evidence / Evidenze: `{{b2_evidence}}`
- Gaps / Azioni: `{{b2_gaps}}`

### B3. Student-centred learning, teaching & assessment

- Answer / Risposta: `{{b3_answer}}`
- Evidence / Evidenze: `{{b3_evidence}}`
- Gaps / Azioni: `{{b3_gaps}}`

### B4. Admission, progression, recognition & certification

- Answer / Risposta: `{{b4_answer}}`
- Evidence / Evidenze: `{{b4_evidence}}`
- Gaps / Azioni: `{{b4_gaps}}`

### B5. Teaching staff competence

- Answer / Risposta: `{{b5_answer}}`
- Evidence / Evidenze: `{{b5_evidence}}`
- Gaps / Azioni: `{{b5_gaps}}`

### B6. Learning resources & student support

- Answer / Risposta: `{{b6_answer}}`
- Evidence / Evidenze: `{{b6_evidence}}`
- Gaps / Azioni: `{{b6_gaps}}`

### B7. Information management

- Answer / Risposta: `{{b7_answer}}`
- Evidence / Evidenze: `{{b7_evidence}}`
- Gaps / Azioni: `{{b7_gaps}}`

### B8. Public information

- Answer / Risposta: `{{b8_answer}}`
- Evidence / Evidenze: `{{b8_evidence}}`
- Gaps / Azioni: `{{b8_gaps}}`

### B9. Periodic review

- Answer / Risposta: `{{b9_answer}}`
- Evidence / Evidenze: `{{b9_evidence}}`
- Gaps / Azioni: `{{b9_gaps}}`

### B10. External QA cycle

- Answer / Risposta: `{{b10_answer}}`
- Evidence / Evidenze: `{{b10_evidence}}`
- Gaps / Azioni: `{{b10_gaps}}`

---

## C) ELM/EDCI data readiness (Accreditation)

Fill what you can today. Missing fields are your backlog.

### C1. Accreditation decision metadata

- Title / Titolo: `{{accreditation_title}}`
- Description / Descrizione: `{{accreditation_description}}`
- Type (controlled list) / Tipo (lista controllata): `{{accreditation_type_uri}}`
  - Hint: ELM EDC profile references controlled lists under `http://publications.europa.eu/resource/dataset/*`.
- Accrediting agent / Ente accreditante: `{{accrediting_agent_name}}`
- Decision (controlled list) / Esito (lista controllata): `{{decision_uri}}`
- Issued date / Data rilascio: `{{issued_at_iso_datetime}}`
- Validity / Validità (optional): `{{valid_from}}` → `{{valid_until}}`
- Review date / Data di riesame: `{{review_date}}`
- Public report URL / URL report pubblico: `{{report_url}}`

**ELM anchors (confirmed in EDC AP docs):** `elm:Accreditation`, `elm:accreditingAgent`, `elm:decision`, `elm:report`, `elm:limitEQFLevel`, `elm:limitField`, `elm:limitJurisdiction`.

### C2. Scope

- Accredited organisation / Organizzazione accreditata: `{{accredited_org_name}}`
- Accredited qualification (optional) / Titolo accreditato (opzionale): `{{accredited_qualification}}`
- EQF levels covered / Livelli EQF coperti: `{{eqf_levels}}`
- Fields covered (ISCED-F) / Ambiti (ISCED-F): `{{iscedf_codes}}`
- Jurisdictions / Giurisdizioni: `{{jurisdictions}}`

---

## Versioning

- v1 — 2026-05-27, initial accreditation self-assessment template; governance checklist + ELM/EDCI metadata readiness.
