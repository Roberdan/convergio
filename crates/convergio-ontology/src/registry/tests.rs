use super::{RegistryError, SchemaRegistry, SchemaSpec};
use crate::{ObjectType, PropertyKey, PropertyType, SchemaVersion, TypeId};
use std::collections::BTreeMap;

fn t(s: &str) -> TypeId {
    s.parse().unwrap()
}

fn k(s: &str) -> PropertyKey {
    s.parse().unwrap()
}

fn prop(id: &str, v: SchemaVersion) -> PropertyType {
    PropertyType {
        id: t(id),
        schema_version: v,
        title: id.to_string(),
        description: None,
        iri: None,
        kind: crate::PropertyKind::String,
    }
}

fn object(id: &str, v: SchemaVersion, required: bool) -> ObjectType {
    let mut props = BTreeMap::new();
    props.insert(
        k("name"),
        crate::ObjectProperty {
            key: k("name"),
            property_type: t("prop.name"),
            required,
            description: None,
        },
    );
    ObjectType {
        id: t(id),
        schema_version: v,
        title: id.to_string(),
        description: None,
        properties: props,
        allow_additional_properties: false,
    }
}

#[test]
fn unchanged_object_requires_patch_bump() {
    let mut reg = SchemaRegistry::new();
    reg.register(
        SchemaSpec::Property(prop("prop.name", SchemaVersion::new(0, 1, 0))),
        false,
        None,
    )
    .unwrap();
    reg.register(
        SchemaSpec::Object(object("edu.student", SchemaVersion::new(0, 1, 0), false)),
        false,
        None,
    )
    .unwrap();

    let next = SchemaSpec::Object(object("edu.student", SchemaVersion::new(0, 2, 0), false));
    let err = reg.register(next, false, None).unwrap_err();

    match err {
        RegistryError::InvalidVersionBump { expected, got, .. } => {
            assert_eq!(expected, SchemaVersion::new(0, 1, 1));
            assert_eq!(got, SchemaVersion::new(0, 2, 0));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn breaking_change_requires_migration_plan() {
    let mut reg = SchemaRegistry::new();
    reg.register(
        SchemaSpec::Property(prop("prop.name", SchemaVersion::new(0, 1, 0))),
        false,
        None,
    )
    .unwrap();

    reg.register(
        SchemaSpec::Object(object("edu.student", SchemaVersion::new(0, 1, 0), false)),
        false,
        None,
    )
    .unwrap();

    let next = SchemaSpec::Object(object("edu.student", SchemaVersion::new(1, 0, 0), true));
    let err = reg.register(next, true, None).unwrap_err();
    assert!(matches!(
        err,
        RegistryError::BreakingRequiresMigration { .. }
    ));
}
