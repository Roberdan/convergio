# convergio-i18n

P5 enforcement — Internationalization first.

Fluent (Mozilla) bundles for every user-facing string in Convergio.
Italian and English are first-class.

## Quickstart

```rust
use convergio_i18n::{Bundle, Locale, detect_locale};

// Pick a locale: --lang flag → CONVERGIO_LANG → LC_ALL → LANG → fallback en
let locale = detect_locale(cli_lang_flag.as_deref());
let bundle = Bundle::new(locale)?;

println!("{}", bundle.t("health-ok", &[("version", env!("CARGO_PKG_VERSION"))]));
println!("{}", bundle.t_n("plan-list-header", 5));
```

## Adding a new locale

1. Add a `Locale::Xx` variant in `src/locale.rs` (and update
   `Locale::ALL`, `tag()`, `from_tag()`).
2. Create `locales/xx/main.ftl` with **every** key from
   `locales/en/main.ftl`.
3. Add a `match` arm in `src/bundle.rs` for the new locale.
4. Run `cargo test -p convergio-i18n --test coverage` — every test in
   the `coverage` integration suite must pass (it asserts every locale
   loads and that each locale has a translation for every key in
   every other locale).

The coverage tests are the i18n equivalent of P4: a locale is **not**
acceptable until every message has a translation. No partial-locale
shipped.

## Naming conventions

- Keys are kebab-case: `plan-created`, `gate-refused-evidence`.
- Section dividers in `.ftl` use `# ----------`.
- Placeholders use Fluent's `{ $variable }`. Never concatenate
  strings.
- Plural-aware messages use Fluent's selector syntax with `{ $count -> ... }`.
