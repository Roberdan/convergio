//! Fluent bundle wrapper: load embedded `.ftl` source for a locale,
//! format messages with placeholders.

use crate::error::{I18nError, Result};
use crate::locale::Locale;
use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use unic_langid::LanguageIdentifier;

/// `.ftl` source for each locale, baked into the binary at compile
/// time. Adding a new locale = new `include_str!` entry + a new
/// `match` arm here + a corresponding `Locale` variant.
const EN_MAIN: &str = include_str!("../locales/en/main.ftl");
const IT_MAIN: &str = include_str!("../locales/it/main.ftl");

/// A loaded message bundle for one locale. Cheap to query but **not**
/// cheap to construct — build one per process and reuse it.
pub struct Bundle {
    inner: FluentBundle<FluentResource>,
    locale: Locale,
}

impl Bundle {
    /// Build a bundle for the given locale.
    pub fn new(locale: Locale) -> Result<Self> {
        let lang_id: LanguageIdentifier = locale
            .tag()
            .parse()
            .map_err(|e| I18nError::Parse(format!("bad locale tag: {e}")))?;
        let mut bundle = FluentBundle::new(vec![lang_id]);

        // Disable Fluent's default Unicode isolation marks; the CLI
        // output is plain text, the marks would render as garbage.
        bundle.set_use_isolating(false);

        let source = match locale {
            Locale::En => EN_MAIN,
            Locale::It => IT_MAIN,
        };
        let res = FluentResource::try_new(source.to_string())
            .map_err(|(_, errs)| I18nError::Parse(format!("ftl parse: {errs:?}")))?;
        bundle
            .add_resource(res)
            .map_err(|errs| I18nError::Parse(format!("ftl add_resource: {errs:?}")))?;
        Ok(Self {
            inner: bundle,
            locale,
        })
    }

    /// The locale this bundle serves.
    pub fn locale(&self) -> Locale {
        self.locale
    }

    /// Format the message for `key` with `args` (placeholder
    /// substitution). Returns the formatted string, or the key itself
    /// if the message is missing — never panics, never returns an
    /// `Err` (i18n failures should not break the CLI).
    pub fn t(&self, key: &str, args: &[(&str, &str)]) -> String {
        let mut fluent_args = FluentArgs::new();
        for (k, v) in args {
            fluent_args.set(*k, FluentValue::from(*v));
        }
        self.format(key, fluent_args)
    }

    /// `t` with a single number placeholder, used for plural-aware
    /// messages like `plan-list-header`.
    pub fn t_n(&self, key: &str, count: i64) -> String {
        let mut args = FluentArgs::new();
        args.set("count", FluentValue::from(count));
        self.format(key, args)
    }

    /// `t_n` plus additional string placeholders — for plural-aware
    /// messages that also need extra variables (e.g. `plan-triage-header`).
    pub fn t_n_with(&self, key: &str, count: i64, extra: &[(&str, &str)]) -> String {
        let mut args = FluentArgs::new();
        args.set("count", FluentValue::from(count));
        for (k, v) in extra {
            args.set(*k, FluentValue::from(*v));
        }
        self.format(key, args)
    }

    /// Shared formatter for `t`, `t_n`, and `t_n_with`. Centralizes
    /// missing-message and Fluent-format-error diagnostics so plural
    /// helpers stay observable on par with `t` (P1 constitution
    /// compliance).
    fn format(&self, key: &str, args: FluentArgs<'_>) -> String {
        let Some(msg) = self.inner.get_message(key) else {
            tracing::warn!(key, "i18n: missing message");
            return key.to_string();
        };
        let Some(pattern) = msg.value() else {
            tracing::warn!(key, "i18n: message has no value");
            return key.to_string();
        };
        let mut errors = vec![];
        let formatted = self.inner.format_pattern(pattern, Some(&args), &mut errors);
        if !errors.is_empty() {
            tracing::warn!(?errors, key, "i18n: format errors");
        }
        formatted.into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_health_ok() {
        let b = Bundle::new(Locale::En).unwrap();
        assert_eq!(
            b.t("health-ok", &[("version", "0.1.0")]),
            "Daemon is healthy. Version: 0.1.0"
        );
    }

    #[test]
    fn italian_health_ok() {
        let b = Bundle::new(Locale::It).unwrap();
        assert_eq!(
            b.t("health-ok", &[("version", "0.1.0")]),
            "Il daemon è attivo. Versione: 0.1.0"
        );
    }

    #[test]
    fn missing_key_returns_key() {
        let b = Bundle::new(Locale::En).unwrap();
        assert_eq!(b.t("does-not-exist", &[]), "does-not-exist");
    }

    #[test]
    fn plural_one() {
        let b = Bundle::new(Locale::It).unwrap();
        assert_eq!(b.t_n("plan-list-header", 1), "Un piano:");
    }

    #[test]
    fn plural_many() {
        let b = Bundle::new(Locale::It).unwrap();
        assert_eq!(b.t_n("plan-list-header", 5), "5 piani:");
    }

    #[test]
    fn english_plural() {
        let b = Bundle::new(Locale::En).unwrap();
        assert_eq!(b.t_n("plan-list-header", 1), "One plan:");
        assert_eq!(b.t_n("plan-list-header", 7), "7 plans:");
    }
}
