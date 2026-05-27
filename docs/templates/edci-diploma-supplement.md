# EDCI-aligned Diploma Supplement template (v1)

> **Purpose:** source template for a Diploma Supplement (human-readable; typically exported to PDF)
> aligned to the **European Learning Model (ELM)** / **EDCI** vocabulary.
>
> **Status:** template only (Convergio does not auto-issue EDCI credentials yet).
>
> **Language:** provide at least English + Italian for user-facing labels (P5).

## Alignment references

- ELM Browser (official docs): https://europa.eu/europass/elm-browser/index.html
- ELM namespace (core vocabulary): http://data.europa.eu/snb/model/elm/
- EDC application profile (SHACL documentation, browse):
  - https://europa.eu/europass/elm-browser/homepage/3-2-0/edc-generic-no-cv_en.html

## How to use

1. Fill the sections below.
2. Render to PDF using your standard pipeline (Pandoc, Typst, Word, etc.).
3. If you also publish a machine-readable payload, prefer an ELM JSON-LD graph:
   - use the same identifiers (person, qualification, awarding body) as the PDF
   - use ELM controlled vocabularies where required (EQF, ISCED-F, countries, etc.)

## Document metadata

- Document ID / Identificativo documento: `{{diploma_supplement_id}}`
- Issue date / Data di rilascio (ISO-8601): `{{issued_at}}`
- Issuing institution / Istituzione emittente: `{{issuer_name}}`
- Version / Versione: `v1`

---

## 1. Information identifying the holder of the qualification

*(IT: Informazioni che identificano il titolare del titolo)*

- Family name / Cognome: `{{holder_family_name}}`
- Given name(s) / Nome(i): `{{holder_given_names}}`
- Date of birth / Data di nascita (YYYY-MM-DD): `{{holder_birth_date}}`
- Student ID / Matricola (if applicable): `{{holder_student_id}}`
- National identifier / Identificativo nazionale (optional): `{{holder_national_id}}`

**ELM anchors (suggested):** `elm:Person`, `adms:identifier`.

---

## 2. Information identifying the qualification

*(IT: Informazioni che identificano il titolo)*

- Name of qualification (original language) / Denominazione del titolo (lingua originale): `{{qualification_title_native}}`
- Name of qualification (EN) / Denominazione del titolo (EN): `{{qualification_title_en}}`
- Main field(s) of study / Principali ambiti di studio (ISCED-F): `{{iscedf_codes}}`
- Awarding institution / Istituzione che conferisce il titolo:
  - Legal name / Denominazione legale: `{{awarding_body_name}}`
  - Legal identifier / Identificativo legale (optional): `{{awarding_body_legal_id}}`
  - Country / Paese: `{{awarding_body_country}}`
- Institution administering studies (if different) / Istituzione che organizza gli studi (se diversa): `{{administering_institution_name}}`
- Language(s) of instruction / Lingua(e) di insegnamento: `{{languages_of_instruction}}`
- Language(s) of assessment / Lingua(e) di valutazione: `{{languages_of_assessment}}`

**ELM anchors (suggested):** `elm:Qualification`, `elm:Organisation`, `elm:AwardingOpportunity`.

---

## 3. Information on the level and duration of the qualification

*(IT: Informazioni sul livello e sulla durata del titolo)*

- Level of qualification / Livello del titolo:
  - EQF level / Livello EQF: `{{eqf_level}}`
  - NQF level (if applicable) / Livello quadro nazionale (se applicabile): `{{nqf_level}}`
- Official length of programme / Durata ufficiale del programma:
  - Nominal duration / Durata nominale: `{{nominal_duration}}`
  - Total ECTS / Totale ECTS: `{{total_ects}}`
- Access requirements / Requisiti di accesso: `{{access_requirements}}`

**ELM anchors (suggested):** `elm:limitEQFLevel` (where applicable), `elm:CreditPoint`.

---

## 4. Information on the programme and the results obtained

*(IT: Informazioni sul programma e sui risultati conseguiti)*

### 4.1 Programme details

Provide either a module table (preferred) or an attached transcript reference.

| Module / Modulo | Code / Codice | ECTS | Grade / Voto | Period / Periodo | Notes / Note |
|---|---:|---:|---:|---|---|
| `{{module_1_name}}` | `{{module_1_code}}` | `{{module_1_ects}}` | `{{module_1_grade}}` | `{{module_1_period}}` | `{{module_1_notes}}` |
| … | … | … | … | … | … |

### 4.2 Grading scheme

- Grading scale / Scala voti: `{{grading_scale}}`
- Passing grade / Soglia di superamento: `{{passing_grade}}`
- Overall classification (if applicable) / Classificazione finale (se applicabile): `{{overall_classification}}`

**ELM anchors (suggested):** `elm:LearningActivity`, `elm:LearningAssessment`, `elm:GradingScheme`, `elm:ResultCategory`.

---

## 5. Information on the function of the qualification

*(IT: Informazioni sulla funzione del titolo)*

- Access to further study / Accesso a studi successivi: `{{access_to_further_study}}`
- Professional status (if applicable) / Stato professionale (se applicabile): `{{professional_status}}`

---

## 6. Additional information

*(IT: Informazioni aggiuntive)*

- Additional information / Informazioni aggiuntive: `{{additional_info}}`
- Further information sources / Fonti informative aggiuntive:
  - Website / Sito web: `{{info_website}}`
  - Email / Email: `{{info_email}}`

---

## 7. Certification of the supplement

*(IT: Certificazione del supplemento)*

- Date / Data (YYYY-MM-DD): `{{certification_date}}`
- Name / Nome: `{{certifier_name}}`
- Capacity / Ruolo: `{{certifier_role}}`
- Signature / Firma: `{{certifier_signature}}`
- Official stamp / Timbro ufficiale: `{{stamp}}`

---

## 8. Information on the national higher education system

*(IT: Informazioni sul sistema nazionale di istruzione superiore)*

Provide a stable description or a stable link to an official description.

- System overview / Panoramica del sistema: `{{national_system_overview}}`
- Official link / Link ufficiale: `{{national_system_link}}`

---

## Versioning

- v1 — 2026-05-27, initial template aligned to ELM/EDCI terminology; human-readable structure follows the standard 8-section Diploma Supplement outline.
