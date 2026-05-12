//! P5 i18n compliance tests for `cvg pr ...` human output.
//!
//! Each `cvg pr` subcommand's human-mode renderer must surface
//! translated strings (CONSTITUTION § P5 — internationalization
//! first). The audit findings for `pr_link.rs:61`, `pr_who.rs:62`,
//! `pr_merge.rs:196`, and `pr_sync.rs:230` flagged the human
//! output as hard-coded English; these tests exercise the i18n
//! contract every `cvg pr` renderer is expected to honour.
//!
//! The tests are structural: they assert that for each user-facing
//! key, both EN and IT bundles return translated strings (not the
//! raw key — which is what `Bundle::t` falls back to when the
//! message is missing) and that the IT translation differs from
//! the EN one. They fail loudly the day someone adds a new English
//! string to a renderer without an Italian companion.

#[cfg(test)]
mod tests {
    use convergio_i18n::{Bundle, Locale};

    fn assert_translated(key: &str, args: &[(&str, &str)]) {
        let en = Bundle::new(Locale::En).expect("EN bundle loads");
        let it = Bundle::new(Locale::It).expect("IT bundle loads");
        let en_msg = en.t(key, args);
        let it_msg = it.t(key, args);
        assert_ne!(
            en_msg, key,
            "EN bundle is missing key `{key}` — Bundle::t fell back to the raw key"
        );
        assert_ne!(
            it_msg, key,
            "IT bundle is missing key `{key}` — Bundle::t fell back to the raw key"
        );
        assert_ne!(
            en_msg, it_msg,
            "IT translation for `{key}` must differ from EN to satisfy P5"
        );
    }

    // Audit finding MEDIUM pr_link.rs:61.
    #[test]
    fn pr_link_human_output_has_it_translation() {
        assert_translated(
            "pr-link-success",
            &[("pr", "42"), ("plan", "abc"), ("repo", "owner/repo")],
        );
    }

    // Audit finding MEDIUM pr_who.rs:62.
    #[test]
    fn pr_who_empty_message_has_it_translation() {
        assert_translated("pr-who-empty", &[("repo", "o/r"), ("pr", "7")]);
    }

    #[test]
    fn pr_who_ownership_message_has_it_translation() {
        assert_translated(
            "pr-who-ownership",
            &[
                ("repo", "o/r"),
                ("pr", "7"),
                ("agent", "agent-1"),
                ("plan", "p"),
                ("task", "t"),
            ],
        );
    }

    // Audit finding MEDIUM pr_merge.rs:196.
    #[test]
    fn pr_merge_header_has_it_translation() {
        assert_translated("pr-merge-header", &[("pr", "1"), ("head", "feat/x")]);
    }

    #[test]
    fn pr_merge_refused_has_it_translation() {
        assert_translated("pr-merge-refused", &[]);
    }

    #[test]
    fn pr_merge_tracked_header_has_it_translation() {
        assert_translated("pr-merge-tracked-header", &[("count", "0")]);
    }

    #[test]
    fn pr_merge_failed_evidence_header_has_it_translation() {
        assert_translated("pr-merge-failed-evidence-header", &[("count", "1")]);
    }

    // Audit finding MEDIUM pr_sync.rs:230.
    #[test]
    fn pr_sync_header_has_it_translation() {
        assert_translated("pr-sync-header", &[("scanned", "1"), ("tracked", "0")]);
    }

    #[test]
    fn pr_sync_transitioned_header_has_it_translation() {
        assert_translated("pr-sync-transitioned-header", &[("count", "0")]);
    }

    #[test]
    fn pr_sync_skipped_header_has_it_translation() {
        assert_translated("pr-sync-skipped-header", &[("count", "0")]);
    }

    #[test]
    fn pr_sync_failed_header_has_it_translation() {
        assert_translated("pr-sync-failed-header", &[("count", "0")]);
    }

    #[test]
    fn pr_sync_link_failures_header_has_it_translation() {
        assert_translated("pr-sync-link-failures-header", &[("count", "0")]);
    }
}
