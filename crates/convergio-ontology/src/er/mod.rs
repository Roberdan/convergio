//! Deterministic **Entity Resolution (ER)** engine (Ontology Runtime W3,
//! ADR-0055).
//!
//! Entity resolution finds object instances that refer to the same
//! real-world entity. This module ships a **fully-wired deterministic
//! resolver**: it groups instances of one [`ObjectType`](crate) that
//! share an exact, normalized blocking key over a configurable set of
//! properties.
//!
//! # Design
//!
//! - [`MatchRule`] declares the blocking/match key for an object type:
//!   the ordered set of property names whose normalized values must all
//!   be equal for two instances to be considered the same entity.
//!   Normalization is fixed (trim + collapse whitespace + lowercase; see
//!   [`normalize`](self::normalize)).
//! - [`EntityResolver`] resolves over the **latest** value of each
//!   property (the `object_properties` event log is collapsed to current
//!   state) and returns [`MatchGroup`]s.
//! - [`MatchStrategy`] is the extension seam: it maps an instance's
//!   current property map to an optional [`MatchKey`]. [`MatchRule`] is
//!   the deterministic implementation; probabilistic/hybrid strategies
//!   (follow-up, ADR-0055) can implement the same trait without touching
//!   the storage wiring.
//!
//! # Explainability and reversibility
//!
//! Every [`MatchGroup`] carries the matched key, its
//! property/value contributions, and a human-readable `explanation` of
//! **why** the members matched (ADR-0055 explainability requirement).
//! Merge decisions are recorded at the data level as a reversible
//! `er:same-as` link event via [`EntityResolver::record_merge`] /
//! [`EntityResolver::record_unmerge`], reusing the append-only
//! `object_links` log so a merge can always be audited and undone.
//!
//! # Scope (this PR)
//!
//! Deterministic resolution only. Probabilistic and hybrid matching, plus
//! the HTTP/CLI surface for ER, are deliberate follow-ups tracked against
//! ADR-0055.

mod normalize;
mod resolver;

use std::collections::BTreeMap;

pub use resolver::{EntityResolver, SAME_AS_LINK_TYPE};

/// A normalized, deterministic blocking key for a single instance.
///
/// `canonical` is the stable composite string used for grouping; `fields`
/// records each `(property, normalized_value)` contribution so callers can
/// explain *why* a group formed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchKey {
    /// Stable composite key string (property/value pairs joined by a unit
    /// separator). Equal canonicals mean "same entity" under the rule.
    pub canonical: String,
    /// Ordered `(property_name, normalized_value)` pairs that produced the
    /// key, in the rule's property order.
    pub fields: Vec<(String, String)>,
}

/// A group of instances the resolver believes refer to the same entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchGroup {
    /// The shared canonical key (see [`MatchKey::canonical`]).
    pub key: String,
    /// The `(property, normalized_value)` pairs the members share.
    pub fields: Vec<(String, String)>,
    /// Member instance ids, sorted ascending for deterministic output.
    pub members: Vec<String>,
    /// Human-readable explanation of why the members matched
    /// (explainability, ADR-0055).
    pub explanation: String,
}

/// Strategy for deriving a [`MatchKey`] from an instance's current
/// property values.
///
/// This is the extension seam for future probabilistic/hybrid matching:
/// such strategies implement [`MatchStrategy`] with their own keying while
/// the deterministic grouping in [`EntityResolver::candidates`] stays
/// unchanged for the exact-key case. [`MatchRule`] is the deterministic
/// implementation shipped here.
pub trait MatchStrategy {
    /// Compute the match key for an instance whose current property values
    /// are `props` (`property_name -> textual_value`).
    ///
    /// Returns `None` when the instance lacks a usable key (a required key
    /// property is absent or normalizes to empty), so it is never grouped
    /// — this is what prevents false-positive merges on sparse data.
    fn match_key(&self, props: &BTreeMap<String, String>) -> Option<MatchKey>;
}

/// A deterministic blocking/match rule over an object type's properties.
///
/// Two instances match when the normalized values of **all**
/// `key_properties` are present and equal. Configure one rule per object
/// type (e.g. exact match on `["email"]`, or composite `["name", "dob"]`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchRule {
    /// Object type the rule applies to (matched against
    /// `object_instances.type`).
    pub object_type: String,
    /// Ordered property names forming the composite key. Order is part of
    /// the rule's identity and of the produced [`MatchKey`].
    pub key_properties: Vec<String>,
}

/// Unit-separator joining `property=value` segments in a canonical key.
/// Chosen because it cannot appear in human text, so distinct property
/// boundaries never collide.
const KEY_SEP: char = '\u{1f}';

impl MatchRule {
    /// Build a rule for `object_type` keyed on `key_properties` (in order).
    pub fn new(
        object_type: impl Into<String>,
        key_properties: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            object_type: object_type.into(),
            key_properties: key_properties.into_iter().map(Into::into).collect(),
        }
    }
}

impl MatchStrategy for MatchRule {
    fn match_key(&self, props: &BTreeMap<String, String>) -> Option<MatchKey> {
        if self.key_properties.is_empty() {
            return None;
        }
        let mut fields = Vec::with_capacity(self.key_properties.len());
        for prop in &self.key_properties {
            let raw = props.get(prop)?;
            let value = normalize::normalize(raw);
            if value.is_empty() {
                return None;
            }
            fields.push((prop.clone(), value));
        }
        let canonical = fields
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join(&KEY_SEP.to_string());
        Some(MatchKey { canonical, fields })
    }
}

pub(crate) use normalize::value_text;

#[cfg(test)]
mod tests {
    use super::*;

    fn props(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn equal_after_normalization_yields_same_key() {
        let rule = MatchRule::new("Person", ["email"]);
        let a = rule
            .match_key(&props(&[("email", " Alice@Example.com ")]))
            .unwrap();
        let b = rule
            .match_key(&props(&[("email", "alice@example.com")]))
            .unwrap();
        assert_eq!(a.canonical, b.canonical);
    }

    #[test]
    fn missing_key_property_yields_no_key() {
        let rule = MatchRule::new("Person", ["email", "name"]);
        assert!(rule.match_key(&props(&[("email", "a@b.c")])).is_none());
    }

    #[test]
    fn empty_value_yields_no_key() {
        let rule = MatchRule::new("Person", ["email"]);
        assert!(rule.match_key(&props(&[("email", "   ")])).is_none());
    }

    #[test]
    fn empty_rule_yields_no_key() {
        let rule = MatchRule::new("Person", Vec::<String>::new());
        assert!(rule.match_key(&props(&[("email", "a@b.c")])).is_none());
    }

    #[test]
    fn composite_key_is_order_stable() {
        let rule = MatchRule::new("Person", ["name", "dob"]);
        let key = rule
            .match_key(&props(&[("dob", "2000-01-01"), ("name", "Ada")]))
            .unwrap();
        assert_eq!(key.fields[0].0, "name");
        assert_eq!(key.fields[1].0, "dob");
    }
}
