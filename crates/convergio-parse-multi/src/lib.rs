//! # convergio-parse-multi
//!
//! Multi-language AST parsing layer for Convergio fleet retrieval (ADR-0038, F2).
//!
//! Wraps `tree-sitter` grammars for TypeScript and Python into a uniform
//! [`Lang`]/[`parse`] interface so the fleet graph builder can extract
//! nodes from heterogeneous repositories without handling grammar details.
//!
//! ## Entry points
//!
//! | Function | Output | Use case |
//! |---|---|---|
//! | [`parse`] | `Vec<ParsedNode>` | Lightweight node list (no graph types) |
//! | [`ts::parse_ts`] | `(Vec<Node>, Vec<Edge>)` | Fleet graph builder (ADR-0038 §5.3) |
//!
//! ## Supported languages
//!
//! | Variant | Grammar crate |
//! |---------|---------------|
//! | [`Lang::TypeScript`] | `tree-sitter-typescript` |
//! | [`Lang::Python`]     | `tree-sitter-python` |
//!
//! ## Migration range
//!
//! 900-999 reserved by ADR-0003 for this crate.

#![forbid(unsafe_code)]

pub mod error;
pub mod lang;
pub mod migrate;
pub mod node;
pub mod parse;
pub mod ts;

pub use error::{ParseError, Result};
pub use lang::Lang;
pub use node::{NodeKind, ParsedNode};
pub use parse::parse;
pub use ts::parse_ts;
