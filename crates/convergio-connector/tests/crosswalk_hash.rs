//! Integration test: crosswalk hashing is deterministic.
#![allow(missing_docs)]

use convergio_connector::Crosswalk;

#[test]
fn crosswalk_hash_is_deterministic_across_parse() {
    let yaml = r#"
connector_id: "http-json"
fields:
  - source: "id"
    property: "Person.external_id"
    source_key: true
  - source: "email"
    property: "Person.email"
"#;

    let (c1, _) = Crosswalk::from_yaml_bytes(yaml.as_bytes()).expect("parse1");
    let (c2, _) = Crosswalk::from_yaml_bytes(yaml.as_bytes()).expect("parse2");

    let h1 = c1.schema_hash().expect("hash1");
    let h2 = c2.schema_hash().expect("hash2");
    assert_eq!(h1, h2);

    let reordered = r#"
connector_id: "http-json"
fields:
  - source: "email"
    property: "Person.email"
  - source: "id"
    property: "Person.external_id"
    source_key: true
"#;
    let (c3, _) = Crosswalk::from_yaml_bytes(reordered.as_bytes()).expect("parse3");
    let h3 = c3.schema_hash().expect("hash3");
    assert_eq!(h1, h3);
}
