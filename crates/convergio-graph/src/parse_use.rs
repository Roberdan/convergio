//! `use` path flattening helpers extracted from [`super::parse`].
//!
//! Kept in a sibling module so `parse.rs` stays comfortably under the
//! 300-line per-file cap (CONSTITUTION § 13) as the visitor grows.

pub(super) fn path_to_string(p: &syn::Path) -> String {
    p.segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}

pub(super) fn flatten_use(tree: &syn::UseTree, prefix: String) -> Vec<String> {
    use syn::UseTree;
    match tree {
        UseTree::Path(p) => {
            let next = if prefix.is_empty() {
                p.ident.to_string()
            } else {
                format!("{prefix}::{}", p.ident)
            };
            flatten_use(&p.tree, next)
        }
        UseTree::Name(n) => {
            if prefix.is_empty() {
                vec![n.ident.to_string()]
            } else {
                vec![format!("{prefix}::{}", n.ident)]
            }
        }
        UseTree::Rename(r) => {
            if prefix.is_empty() {
                vec![r.ident.to_string()]
            } else {
                vec![format!("{prefix}::{}", r.ident)]
            }
        }
        UseTree::Glob(_) => {
            if prefix.is_empty() {
                vec!["*".to_string()]
            } else {
                vec![format!("{prefix}::*")]
            }
        }
        UseTree::Group(g) => g
            .items
            .iter()
            .flat_map(|t| flatten_use(t, prefix.clone()))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flatten_use_handles_groups() {
        let parsed: syn::ItemUse = syn::parse_str("use a::b::{c, d::e};").unwrap();
        let paths = flatten_use(&parsed.tree, String::new());
        assert!(paths.contains(&"a::b::c".to_string()));
        assert!(paths.contains(&"a::b::d::e".to_string()));
    }

    #[test]
    fn path_to_string_joins_segments() {
        let p: syn::Path = syn::parse_str("std::collections::HashMap").unwrap();
        assert_eq!(path_to_string(&p), "std::collections::HashMap");
    }
}
