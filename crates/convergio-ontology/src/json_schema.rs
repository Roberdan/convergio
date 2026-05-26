use crate::{ObjectType, PropertyKind, PropertyType, SchemaRegistry, TypeId};
use serde_json::{json, Map, Value};

/// Errors during JSON-Schema export.
#[derive(Debug, thiserror::Error)]
pub enum JsonSchemaExportError {
    /// The object references a property type that is not present in the registry.
    #[error("unknown property type: {0}")]
    UnknownPropertyType(TypeId),
}

/// Export a deterministic JSON-Schema for an [`ObjectType`].
///
/// Property types are resolved from the registry by ID (latest version).
pub fn export_object_json_schema(
    registry: &SchemaRegistry,
    object: &ObjectType,
) -> Result<Value, JsonSchemaExportError> {
    let mut properties = Map::new();
    let mut required: Vec<String> = Vec::new();

    for (key, slot) in &object.properties {
        let prop_ty = registry
            .latest_property(&slot.property_type)
            .ok_or_else(|| {
                JsonSchemaExportError::UnknownPropertyType(slot.property_type.clone())
            })?;

        let mut schema = property_schema(prop_ty);

        if let Some(desc) = slot.description.as_ref().or(prop_ty.description.as_ref()) {
            if let Value::Object(map) = &mut schema {
                map.insert("description".to_string(), Value::String(desc.clone()));
            }
        }

        properties.insert(key.as_str().to_string(), schema);
        if slot.required {
            required.push(key.as_str().to_string());
        }
    }

    required.sort();

    let mut root = Map::new();
    root.insert(
        "$schema".to_string(),
        Value::String("https://json-schema.org/draft/2020-12/schema".to_string()),
    );
    root.insert(
        "$id".to_string(),
        Value::String(format!(
            "urn:convergio:ontology:{}:{}",
            object.id, object.schema_version
        )),
    );
    root.insert("title".to_string(), Value::String(object.title.clone()));

    if let Some(desc) = &object.description {
        root.insert("description".to_string(), Value::String(desc.clone()));
    }

    root.insert("type".to_string(), Value::String("object".to_string()));
    root.insert("properties".to_string(), Value::Object(properties));

    if !required.is_empty() {
        root.insert(
            "required".to_string(),
            Value::Array(required.into_iter().map(Value::String).collect()),
        );
    }

    if !object.allow_additional_properties {
        root.insert("additionalProperties".to_string(), Value::Bool(false));
    }

    Ok(Value::Object(root))
}

fn property_schema(prop: &PropertyType) -> Value {
    match prop.kind {
        PropertyKind::String => json!({"type": "string"}),
        PropertyKind::Integer => json!({"type": "integer"}),
        PropertyKind::Number => json!({"type": "number"}),
        PropertyKind::Boolean => json!({"type": "boolean"}),
        PropertyKind::Iri => json!({"type": "string", "format": "uri"}),
        PropertyKind::Date => json!({"type": "string", "format": "date"}),
        PropertyKind::DateTime => json!({"type": "string", "format": "date-time"}),
        PropertyKind::Json => json!({}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ObjectProperty, PropertyKey, PropertyType, SchemaVersion, TypeId};
    use std::collections::BTreeMap;

    fn t(s: &str) -> TypeId {
        s.parse().unwrap()
    }

    fn k(s: &str) -> PropertyKey {
        s.parse().unwrap()
    }

    #[test]
    fn export_is_deterministic() {
        let mut reg = SchemaRegistry::new();
        reg.register(
            crate::SchemaSpec::Property(PropertyType {
                id: t("prop.name"),
                schema_version: SchemaVersion::new(0, 1, 0),
                title: "Name".to_string(),
                description: None,
                iri: None,
                kind: PropertyKind::String,
            }),
            false,
            None,
        )
        .unwrap();

        let mut props = BTreeMap::new();
        props.insert(
            k("name"),
            ObjectProperty {
                key: k("name"),
                property_type: t("prop.name"),
                required: true,
                description: None,
            },
        );
        let obj = ObjectType {
            id: t("edu.student"),
            schema_version: SchemaVersion::new(0, 1, 0),
            title: "Student".to_string(),
            description: None,
            properties: props,
            allow_additional_properties: false,
        };

        let s1 = export_object_json_schema(&reg, &obj).unwrap();
        let s2 = export_object_json_schema(&reg, &obj).unwrap();
        assert_eq!(
            serde_json::to_string(&s1).unwrap(),
            serde_json::to_string(&s2).unwrap()
        );
    }
}
