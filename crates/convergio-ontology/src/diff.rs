use crate::{LinkType, ObjectType, PropertyType, SchemaVersion, TypeId};

/// Semver-like classification of a schema change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeClass {
    /// Backwards-compatible metadata-only change.
    Patch,
    /// Backwards-compatible additive change.
    Minor,
    /// Breaking change.
    Major,
}

/// Human-readable, stable explanation of the change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangeReport {
    /// Coarse change class used to validate schema version bumps.
    pub class: ChangeClass,
    /// Whether the change is breaking for consumers.
    pub breaking: bool,
    /// Human-readable reasons (English-only) explaining the classification.
    pub reasons: Vec<String>,
}

impl ChangeReport {
    fn new(class: ChangeClass, breaking: bool, reasons: Vec<String>) -> Self {
        Self {
            class,
            breaking,
            reasons,
        }
    }
}

/// Classify an [`ObjectType`] change into patch/minor/major.
pub fn classify_object_change(old: &ObjectType, new: &ObjectType) -> ChangeReport {
    let mut reasons = Vec::new();

    if old.id != new.id {
        reasons.push("object id changed".to_string());
        return ChangeReport::new(ChangeClass::Major, true, reasons);
    }

    let mut breaking = false;
    let mut class = ChangeClass::Patch;

    for (k, old_prop) in &old.properties {
        match new.properties.get(k) {
            Some(new_prop) => {
                if old_prop.property_type != new_prop.property_type {
                    breaking = true;
                    class = ChangeClass::Major;
                    reasons.push(format!("property '{k}' type reference changed"));
                }
                if !old_prop.required && new_prop.required {
                    breaking = true;
                    class = ChangeClass::Major;
                    reasons.push(format!("property '{k}' became required"));
                }
            }
            None => {
                breaking = true;
                class = ChangeClass::Major;
                reasons.push(format!("property '{k}' removed"));
            }
        }
    }

    for (k, new_prop) in &new.properties {
        if !old.properties.contains_key(k) {
            if new_prop.required {
                breaking = true;
                class = ChangeClass::Major;
                reasons.push(format!("required property '{k}' added"));
            } else if class != ChangeClass::Major {
                class = ChangeClass::Minor;
                reasons.push(format!("optional property '{k}' added"));
            }
        }
    }

    if old.title != new.title {
        reasons.push("title changed".to_string());
    }
    if old.description != new.description {
        reasons.push("description changed".to_string());
    }

    ChangeReport::new(class, breaking, reasons)
}

/// Classify a [`PropertyType`] change into patch/minor/major.
pub fn classify_property_change(old: &PropertyType, new: &PropertyType) -> ChangeReport {
    let mut reasons = Vec::new();

    if old.id != new.id {
        reasons.push("property id changed".to_string());
        return ChangeReport::new(ChangeClass::Major, true, reasons);
    }

    if old.kind != new.kind {
        reasons.push("property kind changed".to_string());
        return ChangeReport::new(ChangeClass::Major, true, reasons);
    }

    if old.iri != new.iri {
        reasons.push("property IRI changed".to_string());
    }
    if old.title != new.title {
        reasons.push("title changed".to_string());
    }
    if old.description != new.description {
        reasons.push("description changed".to_string());
    }

    ChangeReport::new(ChangeClass::Patch, false, reasons)
}

/// Classify a [`LinkType`] change into patch/minor/major.
pub fn classify_link_change(old: &LinkType, new: &LinkType) -> ChangeReport {
    let mut reasons = Vec::new();

    if old.id != new.id {
        reasons.push("link id changed".to_string());
        return ChangeReport::new(ChangeClass::Major, true, reasons);
    }

    let mut breaking = false;
    let mut class = ChangeClass::Patch;

    if old.from != new.from {
        breaking = true;
        class = ChangeClass::Major;
        reasons.push("from changed".to_string());
    }
    if old.to != new.to {
        breaking = true;
        class = ChangeClass::Major;
        reasons.push("to changed".to_string());
    }

    if old.iri != new.iri {
        reasons.push("IRI changed".to_string());
    }
    if old.title != new.title {
        reasons.push("title changed".to_string());
    }
    if old.description != new.description {
        reasons.push("description changed".to_string());
    }

    ChangeReport::new(class, breaking, reasons)
}

/// Compute the exact next version expected for a change class.
pub fn expected_next_version(last: SchemaVersion, class: ChangeClass) -> SchemaVersion {
    match class {
        ChangeClass::Patch => last.next_patch(),
        ChangeClass::Minor => last.next_minor(),
        ChangeClass::Major => last.next_major(),
    }
}

/// Formats a stable kind label (`"object:edu.student"`).
pub fn kind_label(kind: &str, id: &TypeId) -> String {
    format!("{kind}:{id}")
}
