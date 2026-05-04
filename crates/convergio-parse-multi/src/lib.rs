//! # convergio-parse-multi
//!
//! Multi-language AST parsing layer for Convergio fleet retrieval (ADR-0038, F2).
//!
//! Wraps `tree-sitter` grammars for TypeScript and Python into a uniform
//! [`Lang`]/[`parse`] interface so the fleet graph builder can extract
//! nodes from heterogeneous repositories without handling grammar details.
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

pub use error::{ParseError, Result};
pub use lang::Lang;
pub use node::{NodeKind, ParsedNode};
pub use parse::parse;
