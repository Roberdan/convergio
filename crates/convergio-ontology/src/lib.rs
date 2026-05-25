//! Ontology Runtime Core for Convergio.
//!
//! Implements the platform-side primitive described in ADR-0053: a
//! schema registry of typed domain objects, links, and properties
//! that becomes the peer of the Modulor `(task, evidence, gate,
//! audit_row)` tuple. The shape here is `(object, link, property,
//! schema_version)`.
//!
//! # Scope
//!
//! - **In scope (this crate):** `ObjectType`, `LinkType`,
//!   `PropertyType` records, evolution rules, deterministic
//!   JSON-Schema and SHACL export, diff between schema versions.
//! - **Out of scope (verticals own these):** concrete domain
//!   instances. Convergio ships **zero** built-in `ObjectType`
//!   instances. Accelerators such as `convergio-edu`,
//!   `convergio-research`, `convergio-healthcare-compliance`
//!   register their YAML at plan-create time.
//!
//! # Status
//!
//! W1 scaffold only. This file intentionally exposes no public API;
//! later tasks in the same plan add the schema tables (W1 task 2,
//! owned by `convergio-db`), the deterministic exporters (W1 tasks
//! 3 and 4), the `cvg ontology` CLI surface (W1 task 5), and the
//! MCP `ontology.*` actions (W1 task 6).
//!
//! # Determinism
//!
//! Every export produced by this crate MUST be byte-identical across
//! reruns and across machines for identical inputs. The posture
//! mirrors `actions.json` (ADR-0047) and the graph output formats
//! (ADR-0060). Golden tests enforce the invariant per export
//! surface.

#![forbid(unsafe_code)]
