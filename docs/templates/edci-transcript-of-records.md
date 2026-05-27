# EDCI-aligned Transcript of Records template (PDF source) (v1)

> **Purpose:** human-readable Transcript of Records template (typically exported to PDF)
> with a companion **JSON-LD** payload (see `edci-transcript-of-records.jsonld`).
>
> **Status:** template only (Convergio does not auto-issue EDCI credentials yet).
>
> **Language:** provide at least English + Italian for user-facing labels (P5).

## Alignment references

- ELM Browser (official docs): https://europa.eu/europass/elm-browser/index.html
- ELM namespace: http://data.europa.eu/snb/model/elm/
- EDC application profile landing page:
  https://europa.eu/europass/elm-browser/homepage/3-2-0/edc-generic-no-cv_en.html

## Document metadata

- Transcript ID / ID certificato: `{{transcript_id}}`
- Issue date / Data emissione (YYYY-MM-DD): `{{issued_at}}`
- Issuing institution / Istituzione emittente: `{{issuer_name}}`
- Academic year(s) / Anno(i) accademico(i): `{{academic_years}}`

---

## Student / Studente

- Family name / Cognome: `{{holder_family_name}}`
- Given name(s) / Nome(i): `{{holder_given_names}}`
- Date of birth / Data di nascita (YYYY-MM-DD): `{{holder_birth_date}}`
- Student ID / Matricola: `{{holder_student_id}}`

---

## Programme / Corso di studi

- Programme title / Titolo del programma: `{{programme_title}}`
- Qualification pursued / Titolo conseguito (if applicable): `{{qualification_title}}`
- Field(s) (ISCED-F) / Ambiti (ISCED-F): `{{iscedf_codes}}`
- EQF level (if applicable) / Livello EQF (se applicabile): `{{eqf_level}}`

---

## Transcript entries / Voci di carriera

> Add one row per learning activity (course/module), and optionally
> reference learning assessments.

| Learning activity / Attività formativa | Code / Codice | Period / Periodo | ECTS | Grade / Voto | Result date / Data esito | Notes / Note |
|---|---:|---|---:|---:|---|---|
| `{{activity_1_title}}` | `{{activity_1_code}}` | `{{activity_1_period}}` | `{{activity_1_ects}}` | `{{activity_1_grade}}` | `{{activity_1_result_date}}` | `{{activity_1_notes}}` |
| … | … | … | … | … | … | … |

---

## Grading scheme / Sistema di valutazione

- Scale / Scala: `{{grading_scale}}`
- Minimum passing grade / Voto minimo: `{{passing_grade}}`
- Local-to-ECTS grade mapping (if applicable) / Conversione a ECTS (se applicabile): `{{ects_grade_mapping}}`

---

## Totals / Totali

- Total ECTS achieved / Totale ECTS conseguiti: `{{total_ects_achieved}}`
- Overall average (if applicable) / Media (se applicabile): `{{overall_average}}`

---

## Certification / Certificazione

- Date / Data: `{{certification_date}}`
- Name / Nome: `{{certifier_name}}`
- Role / Ruolo: `{{certifier_role}}`
- Signature / Firma: `{{certifier_signature}}`

---

## Versioning

- v1 — 2026-05-27, initial template with a JSON-LD companion aligned to ELM/EDCI vocabulary.
