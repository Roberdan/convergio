use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ReleaseGateRegistry {
    pub(super) schema_version: String,
    #[serde(default)]
    pub(super) tenant: Option<String>,
    #[serde(default)]
    pub(super) generated_at: Option<String>,
    pub(super) algorithms: Vec<AlgorithmEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AlgorithmEntry {
    /// Stable slug used in the public URL path.
    pub(super) slug: String,
    /// Stable action identifier (the AI Action).
    pub(super) action: String,
    pub(super) title: BilingualText,
    pub(super) purpose: BilingualText,
    pub(super) lawful_basis: BilingualText,
    pub(super) data_categories: Vec<BilingualText>,
    pub(super) model: ModelRef,
    pub(super) region: String,
    pub(super) oversight: BilingualText,
    pub(super) risk_class: RiskClass,
    #[serde(default)]
    pub(super) eval_scorecard: Option<Reference>,
    #[serde(default)]
    pub(super) dpia_refs: Vec<Reference>,
    #[serde(default)]
    pub(super) ethics_refs: Vec<Reference>,
    pub(super) limitations: BilingualText,
    pub(super) appeal_contact: AppealContact,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ModelRef {
    pub(super) name: String,
    #[serde(default)]
    pub(super) version: Option<String>,
    #[serde(default)]
    pub(super) provider: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AppealContact {
    #[serde(default)]
    pub(super) email: Option<String>,
    #[serde(default)]
    pub(super) url: Option<String>,
    #[serde(default)]
    pub(super) notes: Option<BilingualText>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Reference {
    pub(super) title: BilingualText,
    pub(super) url: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct BilingualText {
    pub(super) en: String,
    pub(super) it: String,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum RiskClass {
    Low,
    Medium,
    High,
    Critical,
}
