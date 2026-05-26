use crate::diff::{
    classify_link_change, classify_object_change, classify_property_change, expected_next_version,
    kind_label, ChangeClass,
};
use crate::registry::error::RegistryError;
use crate::registry::hash::content_hash_hex;
use crate::registry::model::{RegisteredSchema, SchemaSpec, SchemaSpecMeta};
use crate::{LinkType, ObjectType, PropertyType, SchemaVersion, TypeId};
use std::collections::BTreeMap;
use uuid::Uuid;

/// In-memory versioned registry.
///
/// Persistence lives elsewhere; this crate focuses on policy and deterministic
/// representations.
#[derive(Default)]
pub struct SchemaRegistry {
    objects: BTreeMap<TypeId, BTreeMap<SchemaVersion, ObjectType>>,
    links: BTreeMap<TypeId, BTreeMap<SchemaVersion, LinkType>>,
    properties: BTreeMap<TypeId, BTreeMap<SchemaVersion, PropertyType>>,
}

impl SchemaRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a schema spec.
    ///
    /// Policy:
    /// - The version bump must match the computed change class.
    /// - Breaking changes require `breaking=true` and `migration_plan=Some(_)`.
    pub fn register(
        &mut self,
        spec: SchemaSpec,
        breaking: bool,
        migration_plan: Option<Uuid>,
    ) -> Result<RegisteredSchema, RegistryError> {
        let meta = spec.meta();
        let label = kind_label(meta.kind, &meta.id);

        if let Some(last_version) = self.latest_version(meta.kind, &meta.id) {
            let class = self.classify_change_with_last(&spec)?.class;
            let expected = expected_next_version(last_version, class);
            if meta.schema_version != expected {
                return Err(RegistryError::InvalidVersionBump {
                    kind: meta.kind,
                    id: meta.id,
                    last: last_version,
                    expected,
                    got: meta.schema_version,
                });
            }
        }

        let change = self.classify_change_with_last(&spec)?;
        if change.breaking && (!breaking || migration_plan.is_none()) {
            return Err(RegistryError::BreakingRequiresMigration { label });
        }
        if breaking && migration_plan.is_none() {
            return Err(RegistryError::BreakingRequiresMigration { label });
        }

        let content_hash = content_hash_hex(&spec)?;

        match spec {
            SchemaSpec::Object(v) => {
                self.objects
                    .entry(v.id.clone())
                    .or_default()
                    .insert(v.schema_version, v);
            }
            SchemaSpec::Link(v) => {
                self.links
                    .entry(v.id.clone())
                    .or_default()
                    .insert(v.schema_version, v);
            }
            SchemaSpec::Property(v) => {
                self.properties
                    .entry(v.id.clone())
                    .or_default()
                    .insert(v.schema_version, v);
            }
        }

        Ok(RegisteredSchema {
            kind: meta.kind,
            id: meta.id,
            schema_version: meta.schema_version,
            content_hash,
            breaking,
            migration_plan,
            change_class: change.class,
        })
    }

    /// Get an exact object type version.
    pub fn get_object(&self, id: &TypeId, version: SchemaVersion) -> Option<&ObjectType> {
        self.objects.get(id)?.get(&version)
    }

    /// Get the highest registered object type version.
    pub fn latest_object(&self, id: &TypeId) -> Option<&ObjectType> {
        latest_in_map(self.objects.get(id)?)
    }

    /// Get an exact link type version.
    pub fn get_link(&self, id: &TypeId, version: SchemaVersion) -> Option<&LinkType> {
        self.links.get(id)?.get(&version)
    }

    /// Get the highest registered link type version.
    pub fn latest_link(&self, id: &TypeId) -> Option<&LinkType> {
        latest_in_map(self.links.get(id)?)
    }

    /// Get an exact property type version.
    pub fn get_property(&self, id: &TypeId, version: SchemaVersion) -> Option<&PropertyType> {
        self.properties.get(id)?.get(&version)
    }

    /// Get the highest registered property type version.
    pub fn latest_property(&self, id: &TypeId) -> Option<&PropertyType> {
        latest_in_map(self.properties.get(id)?)
    }

    fn latest_version(&self, kind: &'static str, id: &TypeId) -> Option<SchemaVersion> {
        match kind {
            "object" => latest_version_in_map(self.objects.get(id)?),
            "link" => latest_version_in_map(self.links.get(id)?),
            "property" => latest_version_in_map(self.properties.get(id)?),
            _ => None,
        }
    }

    fn classify_change_with_last(&self, spec: &SchemaSpec) -> Result<ChangeSummary, RegistryError> {
        let SchemaSpecMeta {
            kind,
            id,
            schema_version: _,
        } = spec.meta();

        let last_version = self.latest_version(kind, &id);

        let report = match (spec, last_version) {
            (SchemaSpec::Object(new), Some(last)) => {
                let old = self.get_object(&id, last).ok_or_else(|| {
                    RegistryError::MissingPriorVersion {
                        kind,
                        id: id.clone(),
                        version: last,
                    }
                })?;
                classify_object_change(old, new)
            }
            (SchemaSpec::Property(new), Some(last)) => {
                let old = self
                    .properties
                    .get(&id)
                    .and_then(|m| m.get(&last))
                    .ok_or_else(|| RegistryError::MissingPriorVersion {
                        kind,
                        id: id.clone(),
                        version: last,
                    })?;
                classify_property_change(old, new)
            }
            (SchemaSpec::Link(new), Some(last)) => {
                let old = self
                    .links
                    .get(&id)
                    .and_then(|m| m.get(&last))
                    .ok_or_else(|| RegistryError::MissingPriorVersion {
                        kind,
                        id: id.clone(),
                        version: last,
                    })?;
                classify_link_change(old, new)
            }
            (_, None) => {
                return Ok(ChangeSummary {
                    class: ChangeClass::Minor,
                    breaking: false,
                })
            }
        };

        Ok(ChangeSummary {
            class: report.class,
            breaking: report.breaking,
        })
    }
}

fn latest_in_map<T>(m: &BTreeMap<SchemaVersion, T>) -> Option<&T> {
    m.iter().next_back().map(|(_, v)| v)
}

fn latest_version_in_map<T>(m: &BTreeMap<SchemaVersion, T>) -> Option<SchemaVersion> {
    m.keys().next_back().copied()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ChangeSummary {
    class: ChangeClass,
    breaking: bool,
}
