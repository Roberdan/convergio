//! Query-string helpers used by `cvg graph for-task` to assemble the
//! `/v1/graph/for-task/:id?...` URL. Split out of `graph.rs` so the
//! command file keeps headroom under the 300-line cap (T828d03c
//! audit follow-up).

/// Hints passed to `cvg graph for-task` that translate into query
/// parameters on the `/v1/graph/for-task/:id` route.
pub(super) struct ForTaskHints {
    pub crate_name: Option<String>,
    pub related_crates: Option<String>,
    pub adr_required: Option<String>,
    pub docs_required: Option<String>,
    pub validation_profile: Option<String>,
}

/// Append `name=value&` to `path`, percent-encoding the value, when
/// `value` is `Some`.
pub(super) fn append_query_param(path: &mut String, name: &str, value: Option<&str>) {
    if let Some(value) = value {
        path.push_str(name);
        path.push('=');
        path.push_str(&encode_query_value(value));
        path.push('&');
    }
}

/// Minimal RFC 3986 percent-encoding for query-string values. Kept
/// in-crate so the CLI does not pull in a URL crate just for this.
pub(super) fn encode_query_value(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(byte as char);
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_query_value_keeps_unreserved_and_escapes_others() {
        assert_eq!(encode_query_value("abc-123_~."), "abc-123_~.");
        assert_eq!(encode_query_value("a b"), "a%20b");
        assert_eq!(encode_query_value("a/b"), "a%2Fb");
    }

    #[test]
    fn append_query_param_skips_none() {
        let mut path = String::from("/x?");
        append_query_param(&mut path, "k", None);
        assert_eq!(path, "/x?");
    }

    #[test]
    fn append_query_param_writes_some() {
        let mut path = String::from("/x?");
        append_query_param(&mut path, "k", Some("v "));
        assert_eq!(path, "/x?k=v%20&");
    }
}
