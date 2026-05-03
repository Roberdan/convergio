# Changelog

All notable changes to Convergio will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows Semantic Versioning before 1.0 with explicit
MVP scope notes.

## [Unreleased]

### Added
- **brand:** new `convergio-brand` crate — single source of truth
  for palette (`#FF00B4` magenta, `#00C8FF` cyan), claim
  (*Make machines prove it.*), subline, wordmark, and the boot
  animation (ADR-0037).
- **cli:** `cvg about` — print the brand lockup, claim, version
  and source URL. Plays the boot animation on a TTY; static when
  piped or when `NO_COLOR` / `CONVERGIO_THEME=mono` is set.
- **server:** `convergio start` now plays the brand boot banner
  before binding the port (TTY-only, respects `NO_COLOR`).
- **i18n:** `brand-about-tagline`, `brand-about-source`, and
  `brand-about-help` keys in `en` and `it` bundles.
- **assets:** `assets/branding/` — logo, hex mark, wordmark variants,
  CLI mockup, and the original Bash + Rust demo scripts.

### Changed
- **tui:** wordmark gradient now sources its endpoints from
  `convergio_brand::{MAGENTA, CYAN}` so `cvg dash` matches the
  CLI splash byte-for-byte. Semantic status colours unchanged
  (CONSTITUTION P3).
- **docs:** README opens with the new claim and lockup; AGENTS.md
  workspace member list includes `convergio-brand`; new
  ADR-0037 documents the brand decision.

## [0.3.9](https://github.com/Roberdan/convergio/compare/convergio-v0.3.8...convergio-v0.3.9) (2026-05-03)


### Features

* **cli:** cvg agent spawn — drive vendor cli runners end-to-end ([1f922ee](https://github.com/Roberdan/convergio/commit/1f922ee891a96e35deec20ca324b7c2ce8a414cc))
* **cli:** cvg agent spawn — drive vendor cli runners end-to-end ([4f463ee](https://github.com/Roberdan/convergio/commit/4f463ee544b6225a86f7fb46217c5446304e617e))
* **runner:** vendor-cli runner crate (claude + copilot) ([515914b](https://github.com/Roberdan/convergio/commit/515914b10e216c108767d759d3becceab743dbe9))
* **runner:** vendor-cli runner crate (claude + copilot) ([b030c80](https://github.com/Roberdan/convergio/commit/b030c809d69dc7fc8f4bf35a5e1f1ab053e84f46))

## [0.3.8](https://github.com/Roberdan/convergio/compare/convergio-v0.3.7...convergio-v0.3.8) (2026-05-03)


### Features

* **cli:** real worktree + friction-log checks for cvg session pre-stop ([7ea7f9a](https://github.com/Roberdan/convergio/commit/7ea7f9a4d9beec0ca03dbf2fa2ebd2480d828c82))
* **cli:** real worktree + friction-log checks for cvg session pre-stop ([b13d6ea](https://github.com/Roberdan/convergio/commit/b13d6ea56dcec8b905be12fa881f631bbd5b8c4b))
* **durability:** materialised timing cache + plan_pr_links table ([4842037](https://github.com/Roberdan/convergio/commit/4842037cc5b7c40f291ea38266b878aeeea14c87))
* **durability:** timing cache + plan_pr_links (ADR-0031) ([8217894](https://github.com/Roberdan/convergio/commit/82178946365a88414814052081afef5c9514185e))


### Bug Fixes

* **durability:** write timing cache on close_task_post_hoc too ([c6fde06](https://github.com/Roberdan/convergio/commit/c6fde0610c68ec9135284e86efaf9b7313ca67a8))
* extend the existing UPDATE inside `close_task_post_hoc` to set ([c6fde06](https://github.com/Roberdan/convergio/commit/c6fde0610c68ec9135284e86efaf9b7313ca67a8))

## [0.3.7](https://github.com/Roberdan/convergio/compare/convergio-v0.3.6...convergio-v0.3.7) (2026-05-02)


### Features

* **tui:** accessible palette + scoped master/detail filtering ([9aff186](https://github.com/Roberdan/convergio/commit/9aff18695775952f052b905966e2017694e0be51))
* **tui:** accessible palette + scoped master/detail filtering ([8ce4544](https://github.com/Roberdan/convergio/commit/8ce45447bb328226f4f10e83212953d2ef9b5775))

## [0.3.6](https://github.com/Roberdan/convergio/compare/convergio-v0.3.5...convergio-v0.3.6) (2026-05-02)


### Features

* **tui:** drill-down + plan ordering by status ([263c552](https://github.com/Roberdan/convergio/commit/263c5523a519f7f94d9ee6fe829f282c421b30fe))
* **tui:** drill-down + plan ordering by status ([2b68aa9](https://github.com/Roberdan/convergio/commit/2b68aa9ee129f456de73ff38ac5bdb9d1f006bf1))

## [0.3.5](https://github.com/Roberdan/convergio/compare/convergio-v0.3.4...convergio-v0.3.5) (2026-05-02)


### Features

* **durability:** cvg plan transition + post /v1/plans/:id/transition ([4f6174a](https://github.com/Roberdan/convergio/commit/4f6174a894987501ebabef92811bc2839ad414ee))
* **durability:** cvg plan transition + POST /v1/plans/:id/transition ([7ca87ef](https://github.com/Roberdan/convergio/commit/7ca87ef69ff1aa147fb925612fa248706aa6418b))
* **tui:** shrink convergio wordmark to 2-row half-block ([b14247a](https://github.com/Roberdan/convergio/commit/b14247ab1d1a7e53cd93e047f602253f0366f863))

## [0.3.4](https://github.com/Roberdan/convergio/compare/convergio-v0.3.3...convergio-v0.3.4) (2026-05-02)


### Features

* **graph:** add structured context hints ([#110](https://github.com/Roberdan/convergio/issues/110)) ([4b3fbd0](https://github.com/Roberdan/convergio/commit/4b3fbd0cbc998d77f130da3c42fe73677e4da5bd))

## [0.3.3](https://github.com/Roberdan/convergio/compare/convergio-v0.3.2...convergio-v0.3.3) (2026-05-02)


### Features

* **tui:** ansi shadow wordmark + side-by-side stats layout ([7c77464](https://github.com/Roberdan/convergio/commit/7c77464164b6f89d55274c70307bdb061cdaa525))
* **tui:** ansi shadow wordmark + side-by-side stats layout ([6ae8176](https://github.com/Roberdan/convergio/commit/6ae81769612719c714eafa76dcbcea419b1524d3))
* **tui:** cvg dash — 4-pane htop-style dashboard (adr-0029) ([6b68808](https://github.com/Roberdan/convergio/commit/6b68808031475fd9790c6114a32c298285ca98e3))
* **tui:** cvg dash — 4-pane htop-style dashboard (ADR-0029) ([17f9cd2](https://github.com/Roberdan/convergio/commit/17f9cd26b0f24ce0c02a3e31f072685f40a0c46a))
* **tui:** repo discovery for cvg update + cvg dash prs scope + header banner ([80cb163](https://github.com/Roberdan/convergio/commit/80cb163bc86e9ec7af0814a59eb7cb758878c18e))
* **tui:** repo discovery for cvg update + cvg dash PRs scope + header banner ([d1d33cd](https://github.com/Roberdan/convergio/commit/d1d33cdb925e57c3827bb591644feb6228577f74))


### Bug Fixes

* **cli:** localize update copy warning ([280447c](https://github.com/Roberdan/convergio/commit/280447c6cfe67fe2d84c6821cd146feba23674ee))
* **cli:** localize update copy warning ([e3eda1a](https://github.com/Roberdan/convergio/commit/e3eda1a96723be3b17bb4ea75abc7d393a0b4030))
* **thor:** harden pipeline execution ([63a8c44](https://github.com/Roberdan/convergio/commit/63a8c449e1d6105e00141f91bea1543a651341ac))
* **thor:** harden pipeline execution ([671d8d1](https://github.com/Roberdan/convergio/commit/671d8d1036bfadcc375b1d527c784da81f8562f5))


### Documentation

* **mcp:** regenerate agent guidance ([03f069c](https://github.com/Roberdan/convergio/commit/03f069cdc95048a526181b34b5b39ec6ebbaee20))
* **repo:** align root crate documentation ([00a8f7c](https://github.com/Roberdan/convergio/commit/00a8f7c773b5cb48b3a410a08cf338d81cbe0539))
* **repo:** normalize english agent guidance ([b1d198d](https://github.com/Roberdan/convergio/commit/b1d198db2d642033418b079e6b99498e50a0a8c7))
* **repo:** refresh generated docs after api-mcp ([fb9d257](https://github.com/Roberdan/convergio/commit/fb9d257d3059f0af618d32c150ee5d39d54621a6))
* **repo:** refresh generated docs after bus ([f5e80d9](https://github.com/Roberdan/convergio/commit/f5e80d9851ebf14ec842a64bd3f583972d140dc3))
* **repo:** refresh generated docs after cli-i18n ([3da9258](https://github.com/Roberdan/convergio/commit/3da925832f6d99f0df721b7de6b80bb08150b04e))
* **repo:** refresh generated docs after docs ([d03e21b](https://github.com/Roberdan/convergio/commit/d03e21b5f8815242af3df8c7eb691d189a13702e))
* **repo:** refresh generated docs after durability-perf ([078c7cb](https://github.com/Roberdan/convergio/commit/078c7cba424d16bcbcaf2f9ce926452744b0aa2e))
* **repo:** refresh generated docs after durability-refactor ([48c997c](https://github.com/Roberdan/convergio/commit/48c997c9ec0a7f96546a128b7eccbeba3ef2571a))
* **repo:** refresh generated docs after durability-tests ([dd79c7c](https://github.com/Roberdan/convergio/commit/dd79c7ca475e3189d80d83aaaf3885e8bfc0aaf1))
* **repo:** refresh generated docs after executor ([f307811](https://github.com/Roberdan/convergio/commit/f307811103780fc14630da5163f1894cd50e94f6))
* **repo:** refresh generated docs after graph ([3715707](https://github.com/Roberdan/convergio/commit/371570776f94302068f0d4f45143186dca1046ee))
* **repo:** refresh generated docs after graph-e2e ([7b685e3](https://github.com/Roberdan/convergio/commit/7b685e3b4a2ef4bbb7a950a8a277047bba9fc0ec))
* **repo:** refresh generated docs after lifecycle ([009527b](https://github.com/Roberdan/convergio/commit/009527b8212d89d97db7db3d05f2415cff58873e))
* **repo:** refresh generated docs after thor ([32f748c](https://github.com/Roberdan/convergio/commit/32f748ccf75398707e12800d1fb8083081994fbe))

## [0.3.2](https://github.com/Roberdan/convergio/compare/convergio-v0.3.1...convergio-v0.3.2) (2026-05-02)


### Features

* **cli:** scaffold cvg session pre-stop subcommand ([3f3848a](https://github.com/Roberdan/convergio/commit/3f3848aacfdaf5d9b177cd186943b44e02d60245))
* **cli:** scaffold cvg session pre-stop subcommand (PRD-001 § Artefact 4) ([8443db0](https://github.com/Roberdan/convergio/commit/8443db0afbf8595a2be5758683cdb7f71662a8b4))

## [0.3.1](https://github.com/Roberdan/convergio/compare/convergio-v0.3.0...convergio-v0.3.1) (2026-05-02)


### Features

* **server:** spawn_runner accepts shell, claude, copilot kinds (adr-0028) ([a2bd521](https://github.com/Roberdan/convergio/commit/a2bd5210527154a8be58f9f9125db02daeab8ca8))
* **server:** spawn_runner accepts shell, claude, copilot kinds (ADR-0028) ([df76803](https://github.com/Roberdan/convergio/commit/df76803111a1322b96c81769a956e33d8ca2291b))
* **server:** wire convergio_executor::spawn_loop alongside reaper + watcher ([ab45ddb](https://github.com/Roberdan/convergio/commit/ab45ddb11b64d6f03c1e79c537e6bfbe08f470c7))
* **server:** wire executor spawn_loop alongside reaper + watcher (ADR-0027) ([bbc2ef9](https://github.com/Roberdan/convergio/commit/bbc2ef92830953d630882b3f0ccac71b04831989))

## [0.3.0](https://github.com/Roberdan/convergio/compare/convergio-v0.2.1...convergio-v0.3.0) (2026-05-02)


### ⚠ BREAKING CHANGES

* Action::CompleteTask removed; SCHEMA_VERSION bumped 1 -> 2; cvg task transition no longer accepts done as a target; POST /v1/tasks/:id/transition with target=done now returns 403 done_not_by_thor instead of completing the task. Migration: call cvg validate <plan_id> after submitting; the validator promotes submitted -> done atomically.

### Features

* **api:** add agent action contract ([2aacd08](https://github.com/Roberdan/convergio/commit/2aacd080ba9e31933f4f824316fc748bbbeb703c))
* **bus,server:** implement Layer 2 agent message bus ([3426b38](https://github.com/Roberdan/convergio/commit/3426b38be4d819ed8ad5dea542da02e9b5da6ee3))
* **bus,server:** system.* topic family + /v1/system-messages (ADR-0025) [Wave 0b PR 1/3] ([fab9ff4](https://github.com/Roberdan/convergio/commit/fab9ff49bbe9c334ee186dd7d2502d4b3e29bc5c))
* **bus,server:** system.* topic family + /v1/system-messages route (ADR-0025) ([fccebb3](https://github.com/Roberdan/convergio/commit/fccebb3bd03d6858d06499f5c5d05a267033941f))
* **bus:** poll_filtered with exclude_sender + ADR-0024 (closes F53) ([5a3a0ea](https://github.com/Roberdan/convergio/commit/5a3a0eaf0c86c9acf59ebb973d3f4a1f7410d926))
* **bus:** poll_filtered with exclude_sender + ADR-0024 (F53 closes dogfood gap 7) ([9edd28d](https://github.com/Roberdan/convergio/commit/9edd28de0dba50e521653e7a28231b5121354b7a))
* **capability:** add disable and remove flow ([fedbc27](https://github.com/Roberdan/convergio/commit/fedbc27f7ff5d63d20bdc8a37c0d680244830a79))
* **cli,docs:** cvg-attach skill + cvg setup agent claude [Wave 0b PR 2/3] ([e3fbdf6](https://github.com/Roberdan/convergio/commit/e3fbdf6531a719a4c9320325ab86f61cebfeafe8))
* **cli,docs:** cvg-attach skill + cvg setup agent claude extension [Wave 0b PR 2/3] ([1cbbdc8](https://github.com/Roberdan/convergio/commit/1cbbdc85662c2a621e02b89be90c585c32877e0c))
* **cli:** add cvg task create + extend --output to all task commands (T0+T10) ([9e1ee2f](https://github.com/Roberdan/convergio/commit/9e1ee2f6ff1da3e7a24d35652a7ba7baab0e8482))
* **cli:** add cvg task create + honor --output across task commands (T0+T10) ([6727175](https://github.com/Roberdan/convergio/commit/6727175a995c3223b5c1047774fe27cde4f24f2d))
* **cli:** add local demo and task workflow ([5936a68](https://github.com/Roberdan/convergio/commit/5936a6814d326bf467773ea708432e92c34e0db7))
* **cli:** add local setup and doctor ([85332ea](https://github.com/Roberdan/convergio/commit/85332ea5eb66de2e477c0da47ef0d4ec4d35c01a))
* **cli:** add local status dashboard ([2c8e728](https://github.com/Roberdan/convergio/commit/2c8e7282efb5383ab7f992e01525728c60d04e15))
* **cli:** add productization workflow ([b67ac6d](https://github.com/Roberdan/convergio/commit/b67ac6d7688066927c82336076c3361d23d127c1))
* **cli:** auto-regen markers — test_count + cvg_subcommands + adr_index ([5528ebe](https://github.com/Roberdan/convergio/commit/5528ebea3a5c08e80b92cba4d63f21442cfa93f5))
* **cli:** auto-regen markers — test_count + cvg_subcommands + adr_index ([c78b6d9](https://github.com/Roberdan/convergio/commit/c78b6d973c63f4b8be119f4f55155d4c98031a35))
* **cli:** cvg agent list/show — surface durable agent registry (closes F46 half-wired) ([7d9f73f](https://github.com/Roberdan/convergio/commit/7d9f73f65d9e9dd69e8cc618b698029d7dedbc89))
* **cli:** cvg agent list/show — surface durable agent registry (F46 wired) ([08a4af1](https://github.com/Roberdan/convergio/commit/08a4af108eac582c2f09e287ac21e7e57415c1bc))
* **cli:** cvg bus — read + post the plan-scoped agent message bus ([ef69640](https://github.com/Roberdan/convergio/commit/ef696404c44fa384b1bb59173e72bc45cc165f0e))
* **cli:** cvg bus — read + post the plan-scoped agent message bus ([58b0253](https://github.com/Roberdan/convergio/commit/58b02538058dc3639b42f6d2ceb1f2c92bbf0dfb))
* **cli:** cvg coherence check + ADR frontmatter — closes T1.17 / Tier-2 retrieval ([385dd25](https://github.com/Roberdan/convergio/commit/385dd25dc4e3b3d7fe3b19247a935191a49cd5e9))
* **cli:** cvg pr stack — local PR queue dashboard with conflict detection (T2.03) ([b463657](https://github.com/Roberdan/convergio/commit/b463657477501e9f13b20303bc4181e6e9d59fd8))
* **cli:** cvg pr stack — PR queue dashboard with conflict detection (T2.03) ([fc4e46a](https://github.com/Roberdan/convergio/commit/fc4e46a32053c198b6f4cf019ec43af9b22d2f5b))
* **cli:** cvg pr sync — auto-transition pending tasks on PR merge (T2.04) ([91c2fda](https://github.com/Roberdan/convergio/commit/91c2fda7f392b00b9aa0359b7ea8f28605057d4c))
* **cli:** cvg pr sync — auto-transition pending tasks on PR merge (T2.04) ([ebf0c86](https://github.com/Roberdan/convergio/commit/ebf0c868941c58ddfaf0f95bb90dbff8dbcb1253))
* **cli:** cvg session resume — live cold-start brief from the daemon ([33371db](https://github.com/Roberdan/convergio/commit/33371db597a40476339b403a64ff7ec29872e3bd))
* **cli:** cvg session resume — live cold-start brief from the daemon ([f63df2d](https://github.com/Roberdan/convergio/commit/f63df2d6b085972c4ce8ab16711500a0270ec64c))
* **cli:** cvg status v2 — human-friendly dashboard (closes 9ce7a17c) ([ce4d7b8](https://github.com/Roberdan/convergio/commit/ce4d7b88ae1a51fade6980e9c6459241e0239e1c))
* **cli:** cvg status v2 — human-friendly dashboard (closes 9ce7a17c) ([2c662a2](https://github.com/Roberdan/convergio/commit/2c662a2afbd66733257bcfd56eabca8f5d355973))
* **cli:** cvg update — auto rebuild+restart daemon after main moves ([6066134](https://github.com/Roberdan/convergio/commit/60661349c740c1f5a694a2b5c7aca357010ae3a9))
* **cli:** cvg update — auto rebuild+restart daemon after main moves ([a9e7dcf](https://github.com/Roberdan/convergio/commit/a9e7dcf0ba01bb367a537c0956a462505f16157c))
* **cli:** per-crate AGENTS.md crate_stats AUTO block ([1299b69](https://github.com/Roberdan/convergio/commit/1299b69ac71b477fe24b35756b917d10158f9779))
* **cli:** per-crate AGENTS.md crate_stats AUTO block ([4f450d1](https://github.com/Roberdan/convergio/commit/4f450d1cd2bbe65b0654b38949606b34bcdc39aa))
* **cli:** per-crate AGENTS.md crate_stats AUTO block ([2a5bd85](https://github.com/Roberdan/convergio/commit/2a5bd854e79ac28d08627839d9f87500a3d2553d))
* **coherence:** body-text drift detector — W4b ([c90b691](https://github.com/Roberdan/convergio/commit/c90b691553f4dcb9aaa3b97c421544c323a55da9))
* **coherence:** body-text drift detector — W4b ([b4c444b](https://github.com/Roberdan/convergio/commit/b4c444b45a28b36acb95facba5433912d1914be3))
* **docs:** ADR-0015 + cvg docs regenerate (workspace_members) — W4c ([f52b52e](https://github.com/Roberdan/convergio/commit/f52b52e16e96cb686e171c46989907a20dbb52d9))
* **docs:** ADR-0015 + cvg docs regenerate (workspace_members) — W4c structural fix ([1c82421](https://github.com/Roberdan/convergio/commit/1c82421ba69afcf0adf96185d22756add2b0c9d5))
* **docs:** tier-1 retrieval entry — auto-generated docs/INDEX.md + CI gate (T1.16) ([204b044](https://github.com/Roberdan/convergio/commit/204b04415431dc79ad0085b9381629377e541d70))
* **docs:** tier-1 retrieval entry point — auto-generated docs/INDEX.md (T1.16) ([23786f1](https://github.com/Roberdan/convergio/commit/23786f1eb589624aa13ef87794e6eb2df7caf1e1))
* **durability,docs:** three sacred principles + multilingua NoDebtGate + ZeroWarningsGate ([6c9e7dd](https://github.com/Roberdan/convergio/commit/6c9e7dd76eb0a7d8c16017bcc28ce6add25ee9d8))
* **durability,server:** add Layer 1 reaper loop ([95d608b](https://github.com/Roberdan/convergio/commit/95d608b3ccb10239d9c2578f737dc6be49761404))
* **durability:** add capability registry core ([317176d](https://github.com/Roberdan/convergio/commit/317176daf525f29dfb6daf44ec6e58da689d636a))
* **durability:** add CRDT core storage ([9bd3819](https://github.com/Roberdan/convergio/commit/9bd38199e5cbe455aded79b1e1a0d93b609bbafb))
* **durability:** add durable agent registry ([d34c631](https://github.com/Roberdan/convergio/commit/d34c63130aa5a74defec1ddec2a4de5c34e08fb3))
* **durability:** add workspace resource leases ([ba473a8](https://github.com/Roberdan/convergio/commit/ba473a826dcc4c007957b07220fe4283ea701a6c))
* **durability:** arbitrate workspace merge queue ([c5cbad8](https://github.com/Roberdan/convergio/commit/c5cbad859870083e1793a97a63298a4507219b23))
* **durability:** audit CRDT imports ([4963067](https://github.com/Roberdan/convergio/commit/4963067ea7d533420883a83abea8f1cae60aefcd))
* **durability:** block unresolved CRDT conflicts ([0b87c7f](https://github.com/Roberdan/convergio/commit/0b87c7f7766eeea3aea3a6324c50e84fc38b1099))
* **durability:** close_task_post_hoc + plan rename — implement ADR-0026 ([2320791](https://github.com/Roberdan/convergio/commit/23207917e146c8472c7734ddb678878ae79e39a3))
* **durability:** cvg task retry — failed→pending recovery (closes F38/F49) ([8ff292f](https://github.com/Roberdan/convergio/commit/8ff292ff2235a8064ce9906f5eb8562f2d99e534))
* **durability:** cvg task retry — failed→pending recovery (F49 closes F38) ([e57d48c](https://github.com/Roberdan/convergio/commit/e57d48cae439e4a1cfe31b69a725d147fd6bfcf7))
* **durability:** DELETE /v1/evidence/:id + cvg evidence remove (audited) ([93bb079](https://github.com/Roberdan/convergio/commit/93bb079b000abcbb645b80787dabb72d2b7bf45f))
* **durability:** DELETE /v1/evidence/:id + cvg evidence remove (audited) ([0c68e88](https://github.com/Roberdan/convergio/commit/0c68e884ddf0961337c9c4b1ab296bd150555ff8))
* **durability:** materialize CRDT cells ([49c2924](https://github.com/Roberdan/convergio/commit/49c2924d57b0179eff50689f8ab4f5a43b1b9b02))
* **durability:** NoDebtGate — refuse evidence with debt markers ([7c7ab9f](https://github.com/Roberdan/convergio/commit/7c7ab9f680c910e787783e0b98070662effe3ca5))
* **durability:** P4 NoStubGate — refuse scaffolding-only evidence ([02fb217](https://github.com/Roberdan/convergio/commit/02fb2174f018e123b678bc6b544d0fb19a817fd7))
* **durability:** sync agents.current_task_id with task transitions (F46) ([fc58ec7](https://github.com/Roberdan/convergio/commit/fc58ec7aa40e1b2678cef6146403143d4f3ba99e))
* **durability:** sync agents.current_task_id with task transitions (F46) ([42b8350](https://github.com/Roberdan/convergio/commit/42b8350d3f894d21aaa06de9fb37776afa92e822))
* **durability:** validate workspace patch proposals ([e8e8ce0](https://github.com/Roberdan/convergio/commit/e8e8ce0759068e197c461ad03717a77b9262dfcd))
* **durability:** verify capability signatures ([638c2b0](https://github.com/Roberdan/convergio/commit/638c2b0242ba426d69b1860b5121058dfeccb78f))
* **durability:** WireCheckGate refuses unwired route/cli-path claims (F55-A) ([1119f63](https://github.com/Roberdan/convergio/commit/1119f630b167e8502c45e6c4b4aa7464fe569e9d))
* **durability:** WireCheckGate refuses unwired route/cli-path claims (F55-A) ([3548893](https://github.com/Roberdan/convergio/commit/3548893e76cd077ded03385da927264044c8116b))
* **examples:** claude-skill-quickstart end-to-end demo (T2.01) ([20c8621](https://github.com/Roberdan/convergio/commit/20c862138200a3c2e8a2fb8ffbb0959f0651625f))
* **examples:** claude-skill-quickstart end-to-end demo (T2.01) ([bb9da33](https://github.com/Roberdan/convergio/commit/bb9da33364306be4dbb3eaca106b8ed4a82b9f5d))
* **graph:** convergio-graph + cvg graph build|stats — ADR-0014 PR 14.1 ([5a34908](https://github.com/Roberdan/convergio/commit/5a3490849eb38cce0d489b6a171dbb685b77b48b))
* **graph:** convergio-graph crate + cvg graph build|stats — ADR-0014 PR 14.1 ([c83a00e](https://github.com/Roberdan/convergio/commit/c83a00ee19604dd4c30789d1dcb30d1c74c06d4c))
* **graph:** cvg graph cluster + cvg session resume --task-id (PR 14.3b) ([e72da03](https://github.com/Roberdan/convergio/commit/e72da03b5bb8010e83b5cddddd6027b45db52811))
* **graph:** cvg graph cluster + cvg session resume --task-id (PR 14.3b) ([e1b1550](https://github.com/Roberdan/convergio/commit/e1b1550c62a64ab5e8222a457ea1c9086d32aa07))
* **graph:** cvg graph drift + lefthook post-commit nudge — PR 14.3a ([0ad4f89](https://github.com/Roberdan/convergio/commit/0ad4f89a98720ac7122981ea2c8ea22962beecfa))
* **graph:** cvg graph drift + lefthook post-commit nudge — PR 14.3a ([0f42b24](https://github.com/Roberdan/convergio/commit/0f42b24c20a167eb2754fd8a5fef6e9ce67630e6))
* **graph:** cvg graph for-task + ADR claims edges + lazy mtime fix ([c822bdf](https://github.com/Roberdan/convergio/commit/c822bdf2088b62a61ef0db10705e96d872f8709b))
* **graph:** cvg graph for-task + ADR claims edges + lazy mtime fix — PR 14.2 ([55a2a61](https://github.com/Roberdan/convergio/commit/55a2a617156ffed5a809af7a04332c5f6d6cec8b))
* **i18n,cli,docs:** P5 internationalization first — Italian + English day one ([66a310b](https://github.com/Roberdan/convergio/commit/66a310b0519be9a92d2890d62ae122504ab60fbc))
* **lefthook:** worktree-warn pre-commit hook for CONSTITUTION §15 — closes T1.18 / F28 ([3d8cabc](https://github.com/Roberdan/convergio/commit/3d8cabcafc38475ba947dac17045aa5811fb2364))
* **lifecycle,planner,thor,executor,server,cli:** Layer 3 watcher + Layer 4 ([11e21a9](https://github.com/Roberdan/convergio/commit/11e21a9d0536efcb97a19056f751e4ac67fd2435))
* **lifecycle,server:** implement Layer 3 supervisor + HTTP surface ([01e289d](https://github.com/Roberdan/convergio/commit/01e289de931c9c8f615f742abd35a0e9a4bad238))
* **lifecycle,server:** Layer 3 OS-watcher loop ([9deecb2](https://github.com/Roberdan/convergio/commit/9deecb2208511cae9bc4e8dd0ef127facd0ad31b))
* **mcp:** add local agent bridge ([6b6fc2b](https://github.com/Roberdan/convergio/commit/6b6fc2b6cb8f7f8a2975fc65b6dd6346892475c5))
* **mcp:** expose plan bus actions ([a2ab720](https://github.com/Roberdan/convergio/commit/a2ab7205e7d86281cb4ca7e12a9b61330bae003d))
* only Thor (cvg validate) promotes submitted -&gt; done (ADR-0011) ([09ff57a](https://github.com/Roberdan/convergio/commit/09ff57a92c309ab35b35db82600faef07d6e00c4))
* **planner:** expose planner capability action ([647f895](https://github.com/Roberdan/convergio/commit/647f89543977786b197d0fbedf7c969ab3ae4d9c))
* **plans:** friction log mirror + ADR-0026 vocabulary + post-hoc close (closes F40) ([c254392](https://github.com/Roberdan/convergio/commit/c254392eb8299107e9f072ca4fae82a85287a321))
* **repo:** legibility audit score + CI advisory + CONSTITUTION §16 (T1.15) ([a18ac83](https://github.com/Roberdan/convergio/commit/a18ac83ab07c846aeea8affa745afc4bc4686797))
* **repo:** legibility audit score + CI advisory + CONSTITUTION §16 (T1.15) ([63e6023](https://github.com/Roberdan/convergio/commit/63e6023b073f6728ce6cd358d2330fa031946cbe))
* **scripts:** install-local.sh runs lefthook install — closes T1.21 / F31 ([2d1adea](https://github.com/Roberdan/convergio/commit/2d1adea7eefed43430aa84475261122174762392))
* **server,cli:** wire HTTP layer + cvg CLI + end-to-end test ([13c829f](https://github.com/Roberdan/convergio/commit/13c829f04fb007133decba18df4615848fc0c772))
* **server,docs:** two-session demo + E2E + adversarial reviews + sanitised PRD-001 [Wave 0b PR 3/3] ([f7275cb](https://github.com/Roberdan/convergio/commit/f7275cbd1782c16385019c81c8ebf4c63feffb42))
* **server,docs:** two-session demo + E2E + reviews + sanitised PRD [Wave 0b PR 3/3] ([43c809b](https://github.com/Roberdan/convergio/commit/43c809bbc38d567f2c41058088a92b0897ae584a))
* **server:** add task context packets ([89d5688](https://github.com/Roberdan/convergio/commit/89d56881b236f2a3bad4c706f3c874363ccf04e0))
* **server:** install signed capability packages ([6c84515](https://github.com/Roberdan/convergio/commit/6c84515749e0bbf41875c9769349d6c24c3e82c3))
* **server:** prove local shell runner ([ecfae30](https://github.com/Roberdan/convergio/commit/ecfae30633d86a9e7ffdf85c2fa4866b62252baa))
* **thor:** smart Thor — invoke project pipeline before promoting (T3.02) ([307578f](https://github.com/Roberdan/convergio/commit/307578f9f594c6f8cf1b1ae3f92c894027b510f5))
* **thor:** smart Thor — invoke project pipeline before submitted -&gt; done (T3.02) ([c2a1aa7](https://github.com/Roberdan/convergio/commit/c2a1aa709bc4f25edd987781d233c2294d7a0251))
* **thor:** wave-scoped validation — cvg validate --wave N (T3.06) ([c94cdbd](https://github.com/Roberdan/convergio/commit/c94cdbd1ca89285cf24128af81bc6fcd9e7c4552))
* **thor:** wave-scoped validation — cvg validate --wave N (T3.06) ([0468ed7](https://github.com/Roberdan/convergio/commit/0468ed7aae5e2f05a3640537eabe708aa379afa7))


### Bug Fixes

* **ci:** align public release checks ([dd5a98e](https://github.com/Roberdan/convergio/commit/dd5a98e8ac6cd792a9a583fa99a0eefcdb9ffac5))
* **ci:** capture context-budget script exit code under set -e ([2ad62d9](https://github.com/Roberdan/convergio/commit/2ad62d940b89b95b37ff11d0ba5a06c5fb5fe1d8))
* **ci:** run release workflow for component tags ([313c8ff](https://github.com/Roberdan/convergio/commit/313c8ff6312fb694e9d4c2fb2ed3784ccc7b4825))
* **ci:** trigger lockfile sync for workspace manifest ([c4ec10f](https://github.com/Roberdan/convergio/commit/c4ec10f8a6095edaf5f0727b237037d6faa31cb8))
* **cli:** address Codex review feedback on PRs [#34](https://github.com/Roberdan/convergio/issues/34) + [#35](https://github.com/Roberdan/convergio/issues/35) ([c52a4ed](https://github.com/Roberdan/convergio/commit/c52a4ed491a097cbe5da57752c1180ff26cfee1a))
* **cli:** compact plan_create output-modes test to stay under 300-line cap ([21262bb](https://github.com/Roberdan/convergio/commit/21262bbeeaf668e55103d68332c7a7c29494c1e7))
* **cli:** honor --output on plan create / list / get ([16380ce](https://github.com/Roberdan/convergio/commit/16380ce494d40e755f8705422b172d51bb3b5e6a))
* **cli:** honor --output on plan create + name demo gate-refusal fixtures ([e37c384](https://github.com/Roberdan/convergio/commit/e37c384c1c9f2ad104afcf87aa2605e4de69099e))
* **cli:** keep doctor JSON stderr clean ([c0500b2](https://github.com/Roberdan/convergio/commit/c0500b2ab31e63be7a931b27b12fccaad88087eb))
* **cli:** launchd plist pins PATH + WorkingDirectory (closes F45) ([de1ba84](https://github.com/Roberdan/convergio/commit/de1ba849d377d88bba52af7f24c8e04389b27e5f))
* **cli:** launchd plist pins PATH + WorkingDirectory (closes F45) ([2c85aa2](https://github.com/Roberdan/convergio/commit/2c85aa21f6fe1ce30423589f332d188615ee52b3))
* **cli:** localise cvg pr stack output and validate manifest ([5900a33](https://github.com/Roberdan/convergio/commit/5900a33c69bf4008a5d79a5748195040bad1ab21))
* **cli:** localise cvg pr stack output and validate manifest ([75ffae3](https://github.com/Roberdan/convergio/commit/75ffae3fe8b2904f1f7dec455ed3b87ec561fe98))
* **cli:** resolve 3 Codex review findings on session resume + coherence ([78d1a48](https://github.com/Roberdan/convergio/commit/78d1a48d7162f76652cb2a8a55d344e53c746ac4))
* **cli:** split cli_smoke.rs to satisfy 300-line cap ([8f71670](https://github.com/Roberdan/convergio/commit/8f716701b8ef6423987c9955c76e5b0ef79930b0))
* **coherence:** body-drift walker skips .claude/ + allowlist for future verticals ([bcd3658](https://github.com/Roberdan/convergio/commit/bcd365883555e40da8842bf53b07e92c00e2e3c2))
* **coherence:** body-drift walker skips .claude/ + allowlist for future verticals (PR [#48](https://github.com/Roberdan/convergio/issues/48) follow-up) ([3876d12](https://github.com/Roberdan/convergio/commit/3876d122468e34977a1a71a15f82038ba5505e78))
* **db:** enable SQLite WAL + Normal sync — closes F35 (CI bus-test flake) ([5fe3935](https://github.com/Roberdan/convergio/commit/5fe393545c93a1f93b825e11d241f36c7177ae5b))
* **db:** enable SQLite WAL + Normal sync — closes F35 CI flake ([85bd414](https://github.com/Roberdan/convergio/commit/85bd414a17a61f853bb942a8bfc158a4057a7052))
* **db:** wait for sqlite write locks ([e9b9dcb](https://github.com/Roberdan/convergio/commit/e9b9dcbae0705264583d3c964c438f8f4b30dacf))
* **docs:** pin LC_ALL=C in generate-docs-index for cross-platform sort ([b6b12d9](https://github.com/Roberdan/convergio/commit/b6b12d9f5083eb67d6e29ac419d4ac09a15f38ee))
* **durability,mcp:** validate NewAgent.kind + clarify register vs heartbeat help schema (F52) ([33c0792](https://github.com/Roberdan/convergio/commit/33c0792404cbf2b626912edfd3c7107873e51491))
* **durability,mcp:** validate NewAgent.kind + clarify register vs heartbeat help schema (F52) ([ab68983](https://github.com/Roberdan/convergio/commit/ab68983908098144a39b87ba16129ddb8c7a6c36))
* **durability:** drop stray blank line after sync_agent_current_task ([4d3e596](https://github.com/Roberdan/convergio/commit/4d3e5963463e0b3909f23972a98fa268e05f685d))
* **durability:** harden local audit and gates ([66006e3](https://github.com/Roberdan/convergio/commit/66006e3092d956bdd5e2677714432cf65f148d00))
* **durability:** NoDebt allowlist for debt-topic tasks (F34) ([27f66b5](https://github.com/Roberdan/convergio/commit/27f66b5d958b76e45c984f0db29c6f28048c1e29))
* **durability:** NoDebt allowlist for debt-topic tasks (F34) ([7f8d419](https://github.com/Roberdan/convergio/commit/7f8d4190993b294c514d3793c3446e168b549ab0))
* **durability:** wave-sequence gate treats `failed` as terminal too ([a02823c](https://github.com/Roberdan/convergio/commit/a02823c466e8b7c3769bcb8a5e9ae8151f75fb81))
* **durability:** wave-sequence gate treats failed as terminal ([f0c1014](https://github.com/Roberdan/convergio/commit/f0c1014b96d281664b2941bbeaaff0b132f00a3d))
* **repo:** replace shadowed binaries atomically ([0c1472f](https://github.com/Roberdan/convergio/commit/0c1472f3a90f3e41d2c6abb3423d70173ec6c4e3))
* **scripts:** pin LC_ALL=C in all shell scripts — closes T1.19 / F27 ([0c3cad3](https://github.com/Roberdan/convergio/commit/0c3cad363a09f3565aa357a1b6adbe38b403ac9f))


### Refactoring

* **cli:** split pr.rs + pr_sync.rs under 300-line cap ([7eb3e13](https://github.com/Roberdan/convergio/commit/7eb3e134db82a22c4250a852b122d2287bd56735))
* **repo:** focus runtime on local SQLite ([4e025a6](https://github.com/Roberdan/convergio/commit/4e025a6642e1b5e195642f760706fbe9c4192c58))
* **thor:** split validate_wave tests under 300-line cap ([a3beb96](https://github.com/Roberdan/convergio/commit/a3beb962f14a2f2221633a6b6b4ddbf18888c6d5))


### Documentation

* ADR-0023 observability tier + F51 friction log ([6fca767](https://github.com/Roberdan/convergio/commit/6fca7674a8eb92bdf1e1c8478ffa70d0600f3705))
* **adr:** ADR-0012 OODA-aware validation — the spine for T3.02-T4.05 ([1d4f61b](https://github.com/Roberdan/convergio/commit/1d4f61bb05784480176354bc61529bfdf402e937))
* **adr:** ADR-0012 OODA-aware validation as the spine for T3.02-T4.05 ([c083479](https://github.com/Roberdan/convergio/commit/c083479459893479b0767f1e919651ad9ef558aa))
* **adr:** ADR-0013 split durability + F33/F34 in friction log ([770b1b2](https://github.com/Roberdan/convergio/commit/770b1b2a46df8f1e116b3f8906199babe036e454))
* **adr:** ADR-0026 plan/wave/milestone vocabulary — one source of truth ([f1a563f](https://github.com/Roberdan/convergio/commit/f1a563faba2ed14bff8c813ee1c518ad346cab56))
* **adr:** observability tier (ADR-0023) + F51 friction log ([582fcff](https://github.com/Roberdan/convergio/commit/582fcffa239b11e63973136971b60195f7b5c52b))
* **adr:** promote ADR-0014 + ADR-0015 to accepted ([893587f](https://github.com/Roberdan/convergio/commit/893587fc841fa535e9c1597199998c87071c7a43))
* **adr:** promote ADR-0014 + ADR-0015 to accepted ([680a581](https://github.com/Roberdan/convergio/commit/680a581afad71deeee9a09796b2850e11d7592be))
* **adr:** retire convergio-worktree crate (ADR-0010) ([56d4b51](https://github.com/Roberdan/convergio/commit/56d4b51406fd61831f2f53af706f80aad0ac87be))
* **adr:** retire convergio-worktree crate husk (ADR-0010) ([62e5791](https://github.com/Roberdan/convergio/commit/62e5791aeb0d53f822f817a46175e34a52bcc8c6))
* agent-resume-packet + fresh-eyes test result for clean handoff ([1f4a885](https://github.com/Roberdan/convergio/commit/1f4a8854269cf80038cc7be150be82df0653f325))
* agent-resume-packet + fresh-eyes test result for handoff ([df99782](https://github.com/Roberdan/convergio/commit/df9978247248dc6a6422eb010255a06d76ab6277))
* **agents:** refresh root AGENTS.md (W4a — manual fix of accumulated drift) ([983c1b0](https://github.com/Roberdan/convergio/commit/983c1b02227b7399a4b5693c02227521c196a6cd))
* **agents:** refresh root AGENTS.md to current workspace state ([7b31509](https://github.com/Roberdan/convergio/commit/7b31509d0e7cbdc4ae4a741fc2079564eda07519))
* **bus:** regenerate AUTO crate stats after Wave 0b file split ([138db54](https://github.com/Roberdan/convergio/commit/138db54e28b1c722edab22e0a0a45a94aa58f4cb))
* **constitution:** § 18 agent merge authority — standing authorisation ([d36ac8c](https://github.com/Roberdan/convergio/commit/d36ac8c3ddb732e54f4cc30e0b2a1141d3fd76c2))
* **constitution:** § 18 agent merge authority — standing authorisation ([6c74936](https://github.com/Roberdan/convergio/commit/6c7493644965da65d73e3dd15f8c181c7e5b0a9d))
* **constitution:** § 18 agent merge authority — standing authorisation ([696e61a](https://github.com/Roberdan/convergio/commit/696e61a7d3cc9b718e1792d758b683a562aedde8))
* differentiate enforced/partial/planned + reposition hero around 'auditable refusal' ([8026e0d](https://github.com/Roberdan/convergio/commit/8026e0de4a3b1ca28bf385a1d3819e2303bf939c))
* **plan:** friction log F54 (fmt drift) + F55 (wired check is weak) ([bbae4b9](https://github.com/Roberdan/convergio/commit/bbae4b9ff5ac4b34c12e522b4698d37aae488a78))
* **plan:** friction log F54 (fmt drift) + F55 (wired check is weak) ([e6c87af](https://github.com/Roberdan/convergio/commit/e6c87af287008fc1226b374951ee61c81c8cc7ce))
* **plan:** friction log F62 — main AUTO-block drift + cascading false-failure ([aca193d](https://github.com/Roberdan/convergio/commit/aca193dd4c3b6e63eff5f31c383de2552a98c2a7))
* **plan:** friction log F62 — main AUTO-block drift cascading false-failure ([a826efa](https://github.com/Roberdan/convergio/commit/a826efac27466927b0c13ae7ae89d24d2050c26f))
* **plans:** clarify execution dependencies ([bebd249](https://github.com/Roberdan/convergio/commit/bebd24983df1c526343345f094ae3030308f03e0))
* **plans:** define public push sequence ([1c99b66](https://github.com/Roberdan/convergio/commit/1c99b662f67f1113b59d49cbcc4b58fb1c30a528))
* **plans:** record public push validation ([a97874b](https://github.com/Roberdan/convergio/commit/a97874bb77e3bf70800d5c0bd6ff3678fe16ced7))
* **plans:** record v0.1.x friction log from first dogfood session ([8fed06b](https://github.com/Roberdan/convergio/commit/8fed06b84fa6cb3b0379967986536d7eb7768707))
* **plans:** record v0.1.x friction log from first dogfood session ([d23828a](https://github.com/Roberdan/convergio/commit/d23828aeea0b7ccfd75b0ada05c44702ebc473db))
* **plans:** sync public readiness queue ([90c81a5](https://github.com/Roberdan/convergio/commit/90c81a51f05341cb2eda19c6ee0b07d16d4498a2))
* regen INDEX.md after AGENTS.md line-count drift ([e823304](https://github.com/Roberdan/convergio/commit/e82330400aaae6812cd96d2a693ae47cc77d7ea1))
* regenerate docs/INDEX.md for Wave 0a additions ([347d050](https://github.com/Roberdan/convergio/commit/347d05084d6a75f12d7b8948dc06804e7a376673))
* regenerate INDEX.md (release-please polish) ([59d73e9](https://github.com/Roberdan/convergio/commit/59d73e9a9557321577cfced467662a59e4bf4bb2))
* **release:** align v0.1 public docs ([85b79ce](https://github.com/Roberdan/convergio/commit/85b79ce9b6bacf59be578c38437cd45f5a7799ff))
* **release:** document macos notarization flow ([0d3dde7](https://github.com/Roberdan/convergio/commit/0d3dde7e4f89ea2368ac7efd2ef6b1002dcd3f1d))
* **release:** record public publication ([a1bef7c](https://github.com/Roberdan/convergio/commit/a1bef7c318ae06c20681f79f6f5ff53aaf904eb2))
* **release:** record v0.1 validation ([7f3c380](https://github.com/Roberdan/convergio/commit/7f3c380abdfbf2e6d608c41c5c06702e44982408))
* **release:** refresh notarized artifact metadata ([3588137](https://github.com/Roberdan/convergio/commit/3588137a1a80e19306e58b877c9497e92b23c9f9))
* **repo,server:** refresh CHANGELOG, ROADMAP, server README, status ([558234d](https://github.com/Roberdan/convergio/commit/558234d047f440f302b40bc9bfeec91b9487c6b9))
* **repo:** align public readiness claims ([9d30701](https://github.com/Roberdan/convergio/commit/9d30701fae1c4f75bca109029dfb826e6e0082a3))
* **repo:** codify multi-agent governance ([09729e4](https://github.com/Roberdan/convergio/commit/09729e4a8f2194ddb2ca6f9195dd5b10ea88f5c6))
* **repo:** differentiate enforced/partial/planned in README + CONSTITUTION ([7ab2db3](https://github.com/Roberdan/convergio/commit/7ab2db3a3fa94af712c2d1a350df7611d4ac0a41))
* **repo:** make parallel-agent worktree discipline a constitution rule (§15) ([e396d45](https://github.com/Roberdan/convergio/commit/e396d45195b803ddd2bec0c55aadb4f1d2ada4b6))
* **repo:** require parallel-agent worktree discipline (CONSTITUTION §15) ([f7c509e](https://github.com/Roberdan/convergio/commit/f7c509e5e94087925330e7ac5431e7e8ca204edb))
* **repo:** rewrite hero + vision around 'auditable refusal' mechanism ([68b7b95](https://github.com/Roberdan/convergio/commit/68b7b95d74d925ef92591ab9a9cfc31d1085ec63))
* **repo:** sync ARCHITECTURE with the 17 shipped routes + ADR-0011 paths ([986cba0](https://github.com/Roberdan/convergio/commit/986cba0f2c3906658fdf88be7f34b38b3a292f30))
* **roadmap:** multi-language graph adapters deferred + skip .claude/ in INDEX walker ([1142793](https://github.com/Roberdan/convergio/commit/1142793c8c0b49e9d80edf6763a64f21312754d3))
* **roadmap:** note multi-language graph adapters as deferred (Rust-first) ([88a491c](https://github.com/Roberdan/convergio/commit/88a491caf960491195f6843250439b9558cdd341))
* sync ARCHITECTURE with the 17 shipped routes + ADR-0011 paths ([b2f018f](https://github.com/Roberdan/convergio/commit/b2f018f3d2b173523d2d562440822e785cd072c8))
* wave 0a — long-tail + urbanism baseline ([5d6161a](https://github.com/Roberdan/convergio/commit/5d6161ab92041272c7f42d721d41ab6f24c0be36))
* wave 0a — long-tail + urbanism baseline ([f7c964b](https://github.com/Roberdan/convergio/commit/f7c964bce5f3d21023eac8fd0e42d023ffeed2ee))
* WIP commit template — closes T1.20 / F29 / F30 ([775a617](https://github.com/Roberdan/convergio/commit/775a6173db94be21f9c683a4e93377e9257d9b2f))

## [0.2.1](https://github.com/Roberdan/convergio-local/compare/convergio-local-v0.2.0...convergio-local-v0.2.1) (2026-05-02)


### Features

* **bus,server:** system.* topic family + /v1/system-messages (ADR-0025) [Wave 0b PR 1/3] ([fab9ff4](https://github.com/Roberdan/convergio-local/commit/fab9ff49bbe9c334ee186dd7d2502d4b3e29bc5c))
* **bus,server:** system.* topic family + /v1/system-messages route (ADR-0025) ([fccebb3](https://github.com/Roberdan/convergio-local/commit/fccebb3bd03d6858d06499f5c5d05a267033941f))
* **bus:** poll_filtered with exclude_sender + ADR-0024 (closes F53) ([5a3a0ea](https://github.com/Roberdan/convergio-local/commit/5a3a0eaf0c86c9acf59ebb973d3f4a1f7410d926))
* **bus:** poll_filtered with exclude_sender + ADR-0024 (F53 closes dogfood gap 7) ([9edd28d](https://github.com/Roberdan/convergio-local/commit/9edd28de0dba50e521653e7a28231b5121354b7a))
* **cli,docs:** cvg-attach skill + cvg setup agent claude [Wave 0b PR 2/3] ([e3fbdf6](https://github.com/Roberdan/convergio-local/commit/e3fbdf6531a719a4c9320325ab86f61cebfeafe8))
* **cli,docs:** cvg-attach skill + cvg setup agent claude extension [Wave 0b PR 2/3] ([1cbbdc8](https://github.com/Roberdan/convergio-local/commit/1cbbdc85662c2a621e02b89be90c585c32877e0c))
* **cli:** auto-regen markers — test_count + cvg_subcommands + adr_index ([5528ebe](https://github.com/Roberdan/convergio-local/commit/5528ebea3a5c08e80b92cba4d63f21442cfa93f5))
* **cli:** auto-regen markers — test_count + cvg_subcommands + adr_index ([c78b6d9](https://github.com/Roberdan/convergio-local/commit/c78b6d973c63f4b8be119f4f55155d4c98031a35))
* **cli:** cvg agent list/show — surface durable agent registry (closes F46 half-wired) ([7d9f73f](https://github.com/Roberdan/convergio-local/commit/7d9f73f65d9e9dd69e8cc618b698029d7dedbc89))
* **cli:** cvg agent list/show — surface durable agent registry (F46 wired) ([08a4af1](https://github.com/Roberdan/convergio-local/commit/08a4af108eac582c2f09e287ac21e7e57415c1bc))
* **cli:** cvg bus — read + post the plan-scoped agent message bus ([ef69640](https://github.com/Roberdan/convergio-local/commit/ef696404c44fa384b1bb59173e72bc45cc165f0e))
* **cli:** cvg bus — read + post the plan-scoped agent message bus ([58b0253](https://github.com/Roberdan/convergio-local/commit/58b02538058dc3639b42f6d2ceb1f2c92bbf0dfb))
* **cli:** cvg pr sync — auto-transition pending tasks on PR merge (T2.04) ([91c2fda](https://github.com/Roberdan/convergio-local/commit/91c2fda7f392b00b9aa0359b7ea8f28605057d4c))
* **cli:** cvg pr sync — auto-transition pending tasks on PR merge (T2.04) ([ebf0c86](https://github.com/Roberdan/convergio-local/commit/ebf0c868941c58ddfaf0f95bb90dbff8dbcb1253))
* **cli:** cvg status v2 — human-friendly dashboard (closes 9ce7a17c) ([ce4d7b8](https://github.com/Roberdan/convergio-local/commit/ce4d7b88ae1a51fade6980e9c6459241e0239e1c))
* **cli:** cvg status v2 — human-friendly dashboard (closes 9ce7a17c) ([2c662a2](https://github.com/Roberdan/convergio-local/commit/2c662a2afbd66733257bcfd56eabca8f5d355973))
* **cli:** cvg update — auto rebuild+restart daemon after main moves ([6066134](https://github.com/Roberdan/convergio-local/commit/60661349c740c1f5a694a2b5c7aca357010ae3a9))
* **cli:** cvg update — auto rebuild+restart daemon after main moves ([a9e7dcf](https://github.com/Roberdan/convergio-local/commit/a9e7dcf0ba01bb367a537c0956a462505f16157c))
* **cli:** per-crate AGENTS.md crate_stats AUTO block ([1299b69](https://github.com/Roberdan/convergio-local/commit/1299b69ac71b477fe24b35756b917d10158f9779))
* **cli:** per-crate AGENTS.md crate_stats AUTO block ([4f450d1](https://github.com/Roberdan/convergio-local/commit/4f450d1cd2bbe65b0654b38949606b34bcdc39aa))
* **cli:** per-crate AGENTS.md crate_stats AUTO block ([2a5bd85](https://github.com/Roberdan/convergio-local/commit/2a5bd854e79ac28d08627839d9f87500a3d2553d))
* **coherence:** body-text drift detector — W4b ([c90b691](https://github.com/Roberdan/convergio-local/commit/c90b691553f4dcb9aaa3b97c421544c323a55da9))
* **coherence:** body-text drift detector — W4b ([b4c444b](https://github.com/Roberdan/convergio-local/commit/b4c444b45a28b36acb95facba5433912d1914be3))
* **docs:** ADR-0015 + cvg docs regenerate (workspace_members) — W4c ([f52b52e](https://github.com/Roberdan/convergio-local/commit/f52b52e16e96cb686e171c46989907a20dbb52d9))
* **docs:** ADR-0015 + cvg docs regenerate (workspace_members) — W4c structural fix ([1c82421](https://github.com/Roberdan/convergio-local/commit/1c82421ba69afcf0adf96185d22756add2b0c9d5))
* **durability:** close_task_post_hoc + plan rename — implement ADR-0026 ([2320791](https://github.com/Roberdan/convergio-local/commit/23207917e146c8472c7734ddb678878ae79e39a3))
* **durability:** cvg task retry — failed→pending recovery (closes F38/F49) ([8ff292f](https://github.com/Roberdan/convergio-local/commit/8ff292ff2235a8064ce9906f5eb8562f2d99e534))
* **durability:** cvg task retry — failed→pending recovery (F49 closes F38) ([e57d48c](https://github.com/Roberdan/convergio-local/commit/e57d48cae439e4a1cfe31b69a725d147fd6bfcf7))
* **durability:** DELETE /v1/evidence/:id + cvg evidence remove (audited) ([93bb079](https://github.com/Roberdan/convergio-local/commit/93bb079b000abcbb645b80787dabb72d2b7bf45f))
* **durability:** DELETE /v1/evidence/:id + cvg evidence remove (audited) ([0c68e88](https://github.com/Roberdan/convergio-local/commit/0c68e884ddf0961337c9c4b1ab296bd150555ff8))
* **durability:** sync agents.current_task_id with task transitions (F46) ([fc58ec7](https://github.com/Roberdan/convergio-local/commit/fc58ec7aa40e1b2678cef6146403143d4f3ba99e))
* **durability:** sync agents.current_task_id with task transitions (F46) ([42b8350](https://github.com/Roberdan/convergio-local/commit/42b8350d3f894d21aaa06de9fb37776afa92e822))
* **graph:** convergio-graph + cvg graph build|stats — ADR-0014 PR 14.1 ([5a34908](https://github.com/Roberdan/convergio-local/commit/5a3490849eb38cce0d489b6a171dbb685b77b48b))
* **graph:** convergio-graph crate + cvg graph build|stats — ADR-0014 PR 14.1 ([c83a00e](https://github.com/Roberdan/convergio-local/commit/c83a00ee19604dd4c30789d1dcb30d1c74c06d4c))
* **graph:** cvg graph cluster + cvg session resume --task-id (PR 14.3b) ([e72da03](https://github.com/Roberdan/convergio-local/commit/e72da03b5bb8010e83b5cddddd6027b45db52811))
* **graph:** cvg graph cluster + cvg session resume --task-id (PR 14.3b) ([e1b1550](https://github.com/Roberdan/convergio-local/commit/e1b1550c62a64ab5e8222a457ea1c9086d32aa07))
* **graph:** cvg graph drift + lefthook post-commit nudge — PR 14.3a ([0ad4f89](https://github.com/Roberdan/convergio-local/commit/0ad4f89a98720ac7122981ea2c8ea22962beecfa))
* **graph:** cvg graph drift + lefthook post-commit nudge — PR 14.3a ([0f42b24](https://github.com/Roberdan/convergio-local/commit/0f42b24c20a167eb2754fd8a5fef6e9ce67630e6))
* **graph:** cvg graph for-task + ADR claims edges + lazy mtime fix ([c822bdf](https://github.com/Roberdan/convergio-local/commit/c822bdf2088b62a61ef0db10705e96d872f8709b))
* **graph:** cvg graph for-task + ADR claims edges + lazy mtime fix — PR 14.2 ([55a2a61](https://github.com/Roberdan/convergio-local/commit/55a2a617156ffed5a809af7a04332c5f6d6cec8b))
* **plans:** friction log mirror + ADR-0026 vocabulary + post-hoc close (closes F40) ([c254392](https://github.com/Roberdan/convergio-local/commit/c254392eb8299107e9f072ca4fae82a85287a321))
* **server,docs:** two-session demo + E2E + adversarial reviews + sanitised PRD-001 [Wave 0b PR 3/3] ([f7275cb](https://github.com/Roberdan/convergio-local/commit/f7275cbd1782c16385019c81c8ebf4c63feffb42))
* **server,docs:** two-session demo + E2E + reviews + sanitised PRD [Wave 0b PR 3/3] ([43c809b](https://github.com/Roberdan/convergio-local/commit/43c809bbc38d567f2c41058088a92b0897ae584a))
* **thor:** smart Thor — invoke project pipeline before promoting (T3.02) ([307578f](https://github.com/Roberdan/convergio-local/commit/307578f9f594c6f8cf1b1ae3f92c894027b510f5))
* **thor:** smart Thor — invoke project pipeline before submitted -&gt; done (T3.02) ([c2a1aa7](https://github.com/Roberdan/convergio-local/commit/c2a1aa709bc4f25edd987781d233c2294d7a0251))
* **thor:** wave-scoped validation — cvg validate --wave N (T3.06) ([c94cdbd](https://github.com/Roberdan/convergio-local/commit/c94cdbd1ca89285cf24128af81bc6fcd9e7c4552))
* **thor:** wave-scoped validation — cvg validate --wave N (T3.06) ([0468ed7](https://github.com/Roberdan/convergio-local/commit/0468ed7aae5e2f05a3640537eabe708aa379afa7))


### Bug Fixes

* **cli:** launchd plist pins PATH + WorkingDirectory (closes F45) ([de1ba84](https://github.com/Roberdan/convergio-local/commit/de1ba849d377d88bba52af7f24c8e04389b27e5f))
* **cli:** launchd plist pins PATH + WorkingDirectory (closes F45) ([2c85aa2](https://github.com/Roberdan/convergio-local/commit/2c85aa21f6fe1ce30423589f332d188615ee52b3))
* **cli:** split cli_smoke.rs to satisfy 300-line cap ([8f71670](https://github.com/Roberdan/convergio-local/commit/8f716701b8ef6423987c9955c76e5b0ef79930b0))
* **coherence:** body-drift walker skips .claude/ + allowlist for future verticals ([bcd3658](https://github.com/Roberdan/convergio-local/commit/bcd365883555e40da8842bf53b07e92c00e2e3c2))
* **coherence:** body-drift walker skips .claude/ + allowlist for future verticals (PR [#48](https://github.com/Roberdan/convergio-local/issues/48) follow-up) ([3876d12](https://github.com/Roberdan/convergio-local/commit/3876d122468e34977a1a71a15f82038ba5505e78))
* **durability,mcp:** validate NewAgent.kind + clarify register vs heartbeat help schema (F52) ([33c0792](https://github.com/Roberdan/convergio-local/commit/33c0792404cbf2b626912edfd3c7107873e51491))
* **durability,mcp:** validate NewAgent.kind + clarify register vs heartbeat help schema (F52) ([ab68983](https://github.com/Roberdan/convergio-local/commit/ab68983908098144a39b87ba16129ddb8c7a6c36))
* **durability:** drop stray blank line after sync_agent_current_task ([4d3e596](https://github.com/Roberdan/convergio-local/commit/4d3e5963463e0b3909f23972a98fa268e05f685d))
* **durability:** NoDebt allowlist for debt-topic tasks (F34) ([27f66b5](https://github.com/Roberdan/convergio-local/commit/27f66b5d958b76e45c984f0db29c6f28048c1e29))
* **durability:** NoDebt allowlist for debt-topic tasks (F34) ([7f8d419](https://github.com/Roberdan/convergio-local/commit/7f8d4190993b294c514d3793c3446e168b549ab0))


### Refactoring

* **cli:** split pr.rs + pr_sync.rs under 300-line cap ([7eb3e13](https://github.com/Roberdan/convergio-local/commit/7eb3e134db82a22c4250a852b122d2287bd56735))
* **thor:** split validate_wave tests under 300-line cap ([a3beb96](https://github.com/Roberdan/convergio-local/commit/a3beb962f14a2f2221633a6b6b4ddbf18888c6d5))


### Documentation

* ADR-0023 observability tier + F51 friction log ([6fca767](https://github.com/Roberdan/convergio-local/commit/6fca7674a8eb92bdf1e1c8478ffa70d0600f3705))
* **adr:** ADR-0026 plan/wave/milestone vocabulary — one source of truth ([f1a563f](https://github.com/Roberdan/convergio-local/commit/f1a563faba2ed14bff8c813ee1c518ad346cab56))
* **adr:** observability tier (ADR-0023) + F51 friction log ([582fcff](https://github.com/Roberdan/convergio-local/commit/582fcffa239b11e63973136971b60195f7b5c52b))
* **adr:** promote ADR-0014 + ADR-0015 to accepted ([893587f](https://github.com/Roberdan/convergio-local/commit/893587fc841fa535e9c1597199998c87071c7a43))
* **adr:** promote ADR-0014 + ADR-0015 to accepted ([680a581](https://github.com/Roberdan/convergio-local/commit/680a581afad71deeee9a09796b2850e11d7592be))
* **agents:** refresh root AGENTS.md (W4a — manual fix of accumulated drift) ([983c1b0](https://github.com/Roberdan/convergio-local/commit/983c1b02227b7399a4b5693c02227521c196a6cd))
* **agents:** refresh root AGENTS.md to current workspace state ([7b31509](https://github.com/Roberdan/convergio-local/commit/7b31509d0e7cbdc4ae4a741fc2079564eda07519))
* **bus:** regenerate AUTO crate stats after Wave 0b file split ([138db54](https://github.com/Roberdan/convergio-local/commit/138db54e28b1c722edab22e0a0a45a94aa58f4cb))
* **constitution:** § 18 agent merge authority — standing authorisation ([d36ac8c](https://github.com/Roberdan/convergio-local/commit/d36ac8c3ddb732e54f4cc30e0b2a1141d3fd76c2))
* **constitution:** § 18 agent merge authority — standing authorisation ([6c74936](https://github.com/Roberdan/convergio-local/commit/6c7493644965da65d73e3dd15f8c181c7e5b0a9d))
* **constitution:** § 18 agent merge authority — standing authorisation ([696e61a](https://github.com/Roberdan/convergio-local/commit/696e61a7d3cc9b718e1792d758b683a562aedde8))
* **plan:** friction log F54 (fmt drift) + F55 (wired check is weak) ([bbae4b9](https://github.com/Roberdan/convergio-local/commit/bbae4b9ff5ac4b34c12e522b4698d37aae488a78))
* **plan:** friction log F54 (fmt drift) + F55 (wired check is weak) ([e6c87af](https://github.com/Roberdan/convergio-local/commit/e6c87af287008fc1226b374951ee61c81c8cc7ce))
* **plan:** friction log F62 — main AUTO-block drift + cascading false-failure ([aca193d](https://github.com/Roberdan/convergio-local/commit/aca193dd4c3b6e63eff5f31c383de2552a98c2a7))
* **plan:** friction log F62 — main AUTO-block drift cascading false-failure ([a826efa](https://github.com/Roberdan/convergio-local/commit/a826efac27466927b0c13ae7ae89d24d2050c26f))
* regen INDEX.md after AGENTS.md line-count drift ([e823304](https://github.com/Roberdan/convergio-local/commit/e82330400aaae6812cd96d2a693ae47cc77d7ea1))
* regenerate docs/INDEX.md for Wave 0a additions ([347d050](https://github.com/Roberdan/convergio-local/commit/347d05084d6a75f12d7b8948dc06804e7a376673))
* **roadmap:** multi-language graph adapters deferred + skip .claude/ in INDEX walker ([1142793](https://github.com/Roberdan/convergio-local/commit/1142793c8c0b49e9d80edf6763a64f21312754d3))
* **roadmap:** note multi-language graph adapters as deferred (Rust-first) ([88a491c](https://github.com/Roberdan/convergio-local/commit/88a491caf960491195f6843250439b9558cdd341))
* wave 0a — long-tail + urbanism baseline ([5d6161a](https://github.com/Roberdan/convergio-local/commit/5d6161ab92041272c7f42d721d41ab6f24c0be36))
* wave 0a — long-tail + urbanism baseline ([f7c964b](https://github.com/Roberdan/convergio-local/commit/f7c964bce5f3d21023eac8fd0e42d023ffeed2ee))

## [0.2.0](https://github.com/Roberdan/convergio-local/compare/convergio-local-v0.1.2...convergio-local-v0.2.0) (2026-05-01)


### ⚠ BREAKING CHANGES

* Action::CompleteTask removed; SCHEMA_VERSION bumped 1 -> 2; cvg task transition no longer accepts done as a target; POST /v1/tasks/:id/transition with target=done now returns 403 done_not_by_thor instead of completing the task. Migration: call cvg validate <plan_id> after submitting; the validator promotes submitted -> done atomically.

### Features

* **cli:** add cvg task create + extend --output to all task commands (T0+T10) ([9e1ee2f](https://github.com/Roberdan/convergio-local/commit/9e1ee2f6ff1da3e7a24d35652a7ba7baab0e8482))
* **cli:** add cvg task create + honor --output across task commands (T0+T10) ([6727175](https://github.com/Roberdan/convergio-local/commit/6727175a995c3223b5c1047774fe27cde4f24f2d))
* **cli:** cvg coherence check + ADR frontmatter — closes T1.17 / Tier-2 retrieval ([385dd25](https://github.com/Roberdan/convergio-local/commit/385dd25dc4e3b3d7fe3b19247a935191a49cd5e9))
* **cli:** cvg pr stack — local PR queue dashboard with conflict detection (T2.03) ([b463657](https://github.com/Roberdan/convergio-local/commit/b463657477501e9f13b20303bc4181e6e9d59fd8))
* **cli:** cvg pr stack — PR queue dashboard with conflict detection (T2.03) ([fc4e46a](https://github.com/Roberdan/convergio-local/commit/fc4e46a32053c198b6f4cf019ec43af9b22d2f5b))
* **cli:** cvg session resume — live cold-start brief from the daemon ([33371db](https://github.com/Roberdan/convergio-local/commit/33371db597a40476339b403a64ff7ec29872e3bd))
* **cli:** cvg session resume — live cold-start brief from the daemon ([f63df2d](https://github.com/Roberdan/convergio-local/commit/f63df2d6b085972c4ce8ab16711500a0270ec64c))
* **docs:** tier-1 retrieval entry — auto-generated docs/INDEX.md + CI gate (T1.16) ([204b044](https://github.com/Roberdan/convergio-local/commit/204b04415431dc79ad0085b9381629377e541d70))
* **docs:** tier-1 retrieval entry point — auto-generated docs/INDEX.md (T1.16) ([23786f1](https://github.com/Roberdan/convergio-local/commit/23786f1eb589624aa13ef87794e6eb2df7caf1e1))
* **examples:** claude-skill-quickstart end-to-end demo (T2.01) ([20c8621](https://github.com/Roberdan/convergio-local/commit/20c862138200a3c2e8a2fb8ffbb0959f0651625f))
* **examples:** claude-skill-quickstart end-to-end demo (T2.01) ([bb9da33](https://github.com/Roberdan/convergio-local/commit/bb9da33364306be4dbb3eaca106b8ed4a82b9f5d))
* **lefthook:** worktree-warn pre-commit hook for CONSTITUTION §15 — closes T1.18 / F28 ([3d8cabc](https://github.com/Roberdan/convergio-local/commit/3d8cabcafc38475ba947dac17045aa5811fb2364))
* only Thor (cvg validate) promotes submitted -&gt; done (ADR-0011) ([09ff57a](https://github.com/Roberdan/convergio-local/commit/09ff57a92c309ab35b35db82600faef07d6e00c4))
* **repo:** legibility audit score + CI advisory + CONSTITUTION §16 (T1.15) ([a18ac83](https://github.com/Roberdan/convergio-local/commit/a18ac83ab07c846aeea8affa745afc4bc4686797))
* **repo:** legibility audit score + CI advisory + CONSTITUTION §16 (T1.15) ([63e6023](https://github.com/Roberdan/convergio-local/commit/63e6023b073f6728ce6cd358d2330fa031946cbe))
* **scripts:** install-local.sh runs lefthook install — closes T1.21 / F31 ([2d1adea](https://github.com/Roberdan/convergio-local/commit/2d1adea7eefed43430aa84475261122174762392))


### Bug Fixes

* **ci:** capture context-budget script exit code under set -e ([2ad62d9](https://github.com/Roberdan/convergio-local/commit/2ad62d940b89b95b37ff11d0ba5a06c5fb5fe1d8))
* **cli:** address Codex review feedback on PRs [#34](https://github.com/Roberdan/convergio-local/issues/34) + [#35](https://github.com/Roberdan/convergio-local/issues/35) ([c52a4ed](https://github.com/Roberdan/convergio-local/commit/c52a4ed491a097cbe5da57752c1180ff26cfee1a))
* **cli:** compact plan_create output-modes test to stay under 300-line cap ([21262bb](https://github.com/Roberdan/convergio-local/commit/21262bbeeaf668e55103d68332c7a7c29494c1e7))
* **cli:** honor --output on plan create / list / get ([16380ce](https://github.com/Roberdan/convergio-local/commit/16380ce494d40e755f8705422b172d51bb3b5e6a))
* **cli:** honor --output on plan create + name demo gate-refusal fixtures ([e37c384](https://github.com/Roberdan/convergio-local/commit/e37c384c1c9f2ad104afcf87aa2605e4de69099e))
* **cli:** localise cvg pr stack output and validate manifest ([5900a33](https://github.com/Roberdan/convergio-local/commit/5900a33c69bf4008a5d79a5748195040bad1ab21))
* **cli:** localise cvg pr stack output and validate manifest ([75ffae3](https://github.com/Roberdan/convergio-local/commit/75ffae3fe8b2904f1f7dec455ed3b87ec561fe98))
* **cli:** resolve 3 Codex review findings on session resume + coherence ([78d1a48](https://github.com/Roberdan/convergio-local/commit/78d1a48d7162f76652cb2a8a55d344e53c746ac4))
* **db:** enable SQLite WAL + Normal sync — closes F35 (CI bus-test flake) ([5fe3935](https://github.com/Roberdan/convergio-local/commit/5fe393545c93a1f93b825e11d241f36c7177ae5b))
* **db:** enable SQLite WAL + Normal sync — closes F35 CI flake ([85bd414](https://github.com/Roberdan/convergio-local/commit/85bd414a17a61f853bb942a8bfc158a4057a7052))
* **docs:** pin LC_ALL=C in generate-docs-index for cross-platform sort ([b6b12d9](https://github.com/Roberdan/convergio-local/commit/b6b12d9f5083eb67d6e29ac419d4ac09a15f38ee))
* **durability:** wave-sequence gate treats `failed` as terminal too ([a02823c](https://github.com/Roberdan/convergio-local/commit/a02823c466e8b7c3769bcb8a5e9ae8151f75fb81))
* **durability:** wave-sequence gate treats failed as terminal ([f0c1014](https://github.com/Roberdan/convergio-local/commit/f0c1014b96d281664b2941bbeaaff0b132f00a3d))
* **scripts:** pin LC_ALL=C in all shell scripts — closes T1.19 / F27 ([0c3cad3](https://github.com/Roberdan/convergio-local/commit/0c3cad363a09f3565aa357a1b6adbe38b403ac9f))


### Documentation

* **adr:** ADR-0012 OODA-aware validation — the spine for T3.02-T4.05 ([1d4f61b](https://github.com/Roberdan/convergio-local/commit/1d4f61bb05784480176354bc61529bfdf402e937))
* **adr:** ADR-0012 OODA-aware validation as the spine for T3.02-T4.05 ([c083479](https://github.com/Roberdan/convergio-local/commit/c083479459893479b0767f1e919651ad9ef558aa))
* **adr:** ADR-0013 split durability + F33/F34 in friction log ([770b1b2](https://github.com/Roberdan/convergio-local/commit/770b1b2a46df8f1e116b3f8906199babe036e454))
* **adr:** retire convergio-worktree crate (ADR-0010) ([56d4b51](https://github.com/Roberdan/convergio-local/commit/56d4b51406fd61831f2f53af706f80aad0ac87be))
* **adr:** retire convergio-worktree crate husk (ADR-0010) ([62e5791](https://github.com/Roberdan/convergio-local/commit/62e5791aeb0d53f822f817a46175e34a52bcc8c6))
* agent-resume-packet + fresh-eyes test result for clean handoff ([1f4a885](https://github.com/Roberdan/convergio-local/commit/1f4a8854269cf80038cc7be150be82df0653f325))
* agent-resume-packet + fresh-eyes test result for handoff ([df99782](https://github.com/Roberdan/convergio-local/commit/df9978247248dc6a6422eb010255a06d76ab6277))
* differentiate enforced/partial/planned + reposition hero around 'auditable refusal' ([8026e0d](https://github.com/Roberdan/convergio-local/commit/8026e0de4a3b1ca28bf385a1d3819e2303bf939c))
* **plans:** record v0.1.x friction log from first dogfood session ([8fed06b](https://github.com/Roberdan/convergio-local/commit/8fed06b84fa6cb3b0379967986536d7eb7768707))
* **plans:** record v0.1.x friction log from first dogfood session ([d23828a](https://github.com/Roberdan/convergio-local/commit/d23828aeea0b7ccfd75b0ada05c44702ebc473db))
* **repo:** differentiate enforced/partial/planned in README + CONSTITUTION ([7ab2db3](https://github.com/Roberdan/convergio-local/commit/7ab2db3a3fa94af712c2d1a350df7611d4ac0a41))
* **repo:** make parallel-agent worktree discipline a constitution rule (§15) ([e396d45](https://github.com/Roberdan/convergio-local/commit/e396d45195b803ddd2bec0c55aadb4f1d2ada4b6))
* **repo:** require parallel-agent worktree discipline (CONSTITUTION §15) ([f7c509e](https://github.com/Roberdan/convergio-local/commit/f7c509e5e94087925330e7ac5431e7e8ca204edb))
* **repo:** rewrite hero + vision around 'auditable refusal' mechanism ([68b7b95](https://github.com/Roberdan/convergio-local/commit/68b7b95d74d925ef92591ab9a9cfc31d1085ec63))
* **repo:** sync ARCHITECTURE with the 17 shipped routes + ADR-0011 paths ([986cba0](https://github.com/Roberdan/convergio-local/commit/986cba0f2c3906658fdf88be7f34b38b3a292f30))
* sync ARCHITECTURE with the 17 shipped routes + ADR-0011 paths ([b2f018f](https://github.com/Roberdan/convergio-local/commit/b2f018f3d2b173523d2d562440822e785cd072c8))
* WIP commit template — closes T1.20 / F29 / F30 ([775a617](https://github.com/Roberdan/convergio-local/commit/775a6173db94be21f9c683a4e93377e9257d9b2f))

## [0.1.2](https://github.com/Roberdan/convergio-local/compare/convergio-local-v0.1.1...convergio-local-v0.1.2) (2026-04-30)


### Bug Fixes

* **ci:** run release workflow for component tags ([313c8ff](https://github.com/Roberdan/convergio-local/commit/313c8ff6312fb694e9d4c2fb2ed3784ccc7b4825))

## [0.1.1](https://github.com/Roberdan/convergio-local/compare/convergio-local-v0.1.0...convergio-local-v0.1.1) (2026-04-30)


### Features

* **api:** add agent action contract ([2aacd08](https://github.com/Roberdan/convergio-local/commit/2aacd080ba9e31933f4f824316fc748bbbeb703c))
* **bus,server:** implement Layer 2 agent message bus ([3426b38](https://github.com/Roberdan/convergio-local/commit/3426b38be4d819ed8ad5dea542da02e9b5da6ee3))
* **capability:** add disable and remove flow ([fedbc27](https://github.com/Roberdan/convergio-local/commit/fedbc27f7ff5d63d20bdc8a37c0d680244830a79))
* **cli:** add local demo and task workflow ([5936a68](https://github.com/Roberdan/convergio-local/commit/5936a6814d326bf467773ea708432e92c34e0db7))
* **cli:** add local setup and doctor ([85332ea](https://github.com/Roberdan/convergio-local/commit/85332ea5eb66de2e477c0da47ef0d4ec4d35c01a))
* **cli:** add local status dashboard ([2c8e728](https://github.com/Roberdan/convergio-local/commit/2c8e7282efb5383ab7f992e01525728c60d04e15))
* **cli:** add productization workflow ([b67ac6d](https://github.com/Roberdan/convergio-local/commit/b67ac6d7688066927c82336076c3361d23d127c1))
* **durability,docs:** three sacred principles + multilingua NoDebtGate + ZeroWarningsGate ([6c9e7dd](https://github.com/Roberdan/convergio-local/commit/6c9e7dd76eb0a7d8c16017bcc28ce6add25ee9d8))
* **durability,server:** add Layer 1 reaper loop ([95d608b](https://github.com/Roberdan/convergio-local/commit/95d608b3ccb10239d9c2578f737dc6be49761404))
* **durability:** add capability registry core ([317176d](https://github.com/Roberdan/convergio-local/commit/317176daf525f29dfb6daf44ec6e58da689d636a))
* **durability:** add CRDT core storage ([9bd3819](https://github.com/Roberdan/convergio-local/commit/9bd38199e5cbe455aded79b1e1a0d93b609bbafb))
* **durability:** add durable agent registry ([d34c631](https://github.com/Roberdan/convergio-local/commit/d34c63130aa5a74defec1ddec2a4de5c34e08fb3))
* **durability:** add workspace resource leases ([ba473a8](https://github.com/Roberdan/convergio-local/commit/ba473a826dcc4c007957b07220fe4283ea701a6c))
* **durability:** arbitrate workspace merge queue ([c5cbad8](https://github.com/Roberdan/convergio-local/commit/c5cbad859870083e1793a97a63298a4507219b23))
* **durability:** audit CRDT imports ([4963067](https://github.com/Roberdan/convergio-local/commit/4963067ea7d533420883a83abea8f1cae60aefcd))
* **durability:** block unresolved CRDT conflicts ([0b87c7f](https://github.com/Roberdan/convergio-local/commit/0b87c7f7766eeea3aea3a6324c50e84fc38b1099))
* **durability:** materialize CRDT cells ([49c2924](https://github.com/Roberdan/convergio-local/commit/49c2924d57b0179eff50689f8ab4f5a43b1b9b02))
* **durability:** NoDebtGate — refuse evidence with debt markers ([7c7ab9f](https://github.com/Roberdan/convergio-local/commit/7c7ab9f680c910e787783e0b98070662effe3ca5))
* **durability:** P4 NoStubGate — refuse scaffolding-only evidence ([02fb217](https://github.com/Roberdan/convergio-local/commit/02fb2174f018e123b678bc6b544d0fb19a817fd7))
* **durability:** validate workspace patch proposals ([e8e8ce0](https://github.com/Roberdan/convergio-local/commit/e8e8ce0759068e197c461ad03717a77b9262dfcd))
* **durability:** verify capability signatures ([638c2b0](https://github.com/Roberdan/convergio-local/commit/638c2b0242ba426d69b1860b5121058dfeccb78f))
* **i18n,cli,docs:** P5 internationalization first — Italian + English day one ([66a310b](https://github.com/Roberdan/convergio-local/commit/66a310b0519be9a92d2890d62ae122504ab60fbc))
* **lifecycle,planner,thor,executor,server,cli:** Layer 3 watcher + Layer 4 ([11e21a9](https://github.com/Roberdan/convergio-local/commit/11e21a9d0536efcb97a19056f751e4ac67fd2435))
* **lifecycle,server:** implement Layer 3 supervisor + HTTP surface ([01e289d](https://github.com/Roberdan/convergio-local/commit/01e289de931c9c8f615f742abd35a0e9a4bad238))
* **lifecycle,server:** Layer 3 OS-watcher loop ([9deecb2](https://github.com/Roberdan/convergio-local/commit/9deecb2208511cae9bc4e8dd0ef127facd0ad31b))
* **mcp:** add local agent bridge ([6b6fc2b](https://github.com/Roberdan/convergio-local/commit/6b6fc2b6cb8f7f8a2975fc65b6dd6346892475c5))
* **mcp:** expose plan bus actions ([a2ab720](https://github.com/Roberdan/convergio-local/commit/a2ab7205e7d86281cb4ca7e12a9b61330bae003d))
* **planner:** expose planner capability action ([647f895](https://github.com/Roberdan/convergio-local/commit/647f89543977786b197d0fbedf7c969ab3ae4d9c))
* **server,cli:** wire HTTP layer + cvg CLI + end-to-end test ([13c829f](https://github.com/Roberdan/convergio-local/commit/13c829f04fb007133decba18df4615848fc0c772))
* **server:** add task context packets ([89d5688](https://github.com/Roberdan/convergio-local/commit/89d56881b236f2a3bad4c706f3c874363ccf04e0))
* **server:** install signed capability packages ([6c84515](https://github.com/Roberdan/convergio-local/commit/6c84515749e0bbf41875c9769349d6c24c3e82c3))
* **server:** prove local shell runner ([ecfae30](https://github.com/Roberdan/convergio-local/commit/ecfae30633d86a9e7ffdf85c2fa4866b62252baa))


### Bug Fixes

* **ci:** align public release checks ([dd5a98e](https://github.com/Roberdan/convergio-local/commit/dd5a98e8ac6cd792a9a583fa99a0eefcdb9ffac5))
* **ci:** trigger lockfile sync for workspace manifest ([c4ec10f](https://github.com/Roberdan/convergio-local/commit/c4ec10f8a6095edaf5f0727b237037d6faa31cb8))
* **cli:** keep doctor JSON stderr clean ([c0500b2](https://github.com/Roberdan/convergio-local/commit/c0500b2ab31e63be7a931b27b12fccaad88087eb))
* **db:** wait for sqlite write locks ([e9b9dcb](https://github.com/Roberdan/convergio-local/commit/e9b9dcbae0705264583d3c964c438f8f4b30dacf))
* **durability:** harden local audit and gates ([66006e3](https://github.com/Roberdan/convergio-local/commit/66006e3092d956bdd5e2677714432cf65f148d00))
* **repo:** replace shadowed binaries atomically ([0c1472f](https://github.com/Roberdan/convergio-local/commit/0c1472f3a90f3e41d2c6abb3423d70173ec6c4e3))


### Refactoring

* **repo:** focus runtime on local SQLite ([4e025a6](https://github.com/Roberdan/convergio-local/commit/4e025a6642e1b5e195642f760706fbe9c4192c58))


### Documentation

* **plans:** clarify execution dependencies ([bebd249](https://github.com/Roberdan/convergio-local/commit/bebd24983df1c526343345f094ae3030308f03e0))
* **plans:** define public push sequence ([1c99b66](https://github.com/Roberdan/convergio-local/commit/1c99b662f67f1113b59d49cbcc4b58fb1c30a528))
* **plans:** record public push validation ([a97874b](https://github.com/Roberdan/convergio-local/commit/a97874bb77e3bf70800d5c0bd6ff3678fe16ced7))
* **plans:** sync public readiness queue ([90c81a5](https://github.com/Roberdan/convergio-local/commit/90c81a51f05341cb2eda19c6ee0b07d16d4498a2))
* **release:** align v0.1 public docs ([85b79ce](https://github.com/Roberdan/convergio-local/commit/85b79ce9b6bacf59be578c38437cd45f5a7799ff))
* **release:** document macos notarization flow ([0d3dde7](https://github.com/Roberdan/convergio-local/commit/0d3dde7e4f89ea2368ac7efd2ef6b1002dcd3f1d))
* **release:** record public publication ([a1bef7c](https://github.com/Roberdan/convergio-local/commit/a1bef7c318ae06c20681f79f6f5ff53aaf904eb2))
* **release:** record v0.1 validation ([7f3c380](https://github.com/Roberdan/convergio-local/commit/7f3c380abdfbf2e6d608c41c5c06702e44982408))
* **release:** refresh notarized artifact metadata ([3588137](https://github.com/Roberdan/convergio-local/commit/3588137a1a80e19306e58b877c9497e92b23c9f9))
* **repo,server:** refresh CHANGELOG, ROADMAP, server README, status ([558234d](https://github.com/Roberdan/convergio-local/commit/558234d047f440f302b40bc9bfeec91b9487c6b9))
* **repo:** align public readiness claims ([9d30701](https://github.com/Roberdan/convergio-local/commit/9d30701fae1c4f75bca109029dfb826e6e0082a3))
* **repo:** codify multi-agent governance ([09729e4](https://github.com/Roberdan/convergio-local/commit/09729e4a8f2194ddb2ca6f9195dd5b10ea88f5c6))

## [Unreleased]

No unreleased changes.

## [0.1.0] - 2026-04-30

### Added

- Initial Convergio Local workspace, with layered Rust crates for DB,
  durability, bus, lifecycle, server, CLI, planner, validator and executor.
- SQLite-backed local daemon, localhost HTTP API, pure HTTP `cvg` CLI and
  one-command local install flow.
- Layer 1 durability: plans, tasks, evidence, gates, reaper and
  hash-chained audit verification.
- Layer 2 bus: persistent local plan-scoped messages with publish, poll and
  ack actions.
- Layer 3 lifecycle: local process spawn, heartbeat and watcher.
- Layer 4 reference flow: planner, executor tick, Thor validator and
  `planner.solve` capability-gated action.
- Server-side gate pipeline, including evidence, wave sequencing, no-debt,
  no-stub, no-secrets and zero-warning gates.
- Guided `cvg demo`, local task/evidence commands, service management,
  setup, doctor diagnostics, MCP logs and `cvg mcp tail`.
- Shared typed agent action contract and stdio MCP bridge with
  `convergio.help` and `convergio.act`.
- CRDT storage foundation for multi-actor row/column state.
- Workspace coordination foundation: resources, leases, patch proposals,
  merge queue arbitration and conflict reporting.
- Durable agent registry, task context packets and plan-scoped bus actions
  for multi-agent coordination through the daemon.
- Local capability registry, Ed25519 signature verification, signed local
  `install-file`, disable and remove safety.
- Constrained local shell runner proof through `spawn_runner`.
- English and Italian Fluent bundles with coverage tests.
- Release artifact workflow, local packaging script, macOS signing and
  notarization documentation.
- Project docs: README, Architecture, Constitution, Roadmap, Security,
  Contributing, Code of Conduct, ADRs and public readiness plan.
- Convergio Community License v1.3 (source-available, aligned with the
  legacy `github.com/Roberdan/convergio` repo).

### Changed

- Repositioned the project as a **single-user, local-first, SQLite-only**
  runtime.
- Removed remote deployment and account-model language from current
  documentation.
- Removed the legacy plan scope field from the plan model, schema, API
  and CLI.
- Added a minimal `convergio start` command parser so `convergio --help`
  works and the documented quickstart is real.
- Removed the unused scaffold-only worktree crate from the workspace.
- Updated README, Architecture, Constitution, Security, Roadmap, ADR
  references and crate READMEs around the focused local MVP.
