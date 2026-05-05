# Changelog

All notable changes to Convergio will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows Semantic Versioning before 1.0 with explicit
MVP scope notes.

## [Unreleased]

### Breaking Changes

* **mcp:** `heartbeat_agent` and `retire_agent` MCP actions now accept `id` as the
  canonical field for the agent's own primary key; `agent_id` is a deprecated alias
  that emits a `tracing::warn` and will be removed in 0.4.0 (ADR-0043, C2 fix).

### Added

* **mcp:** ADR-0043 — API consistency: `id` for entity-self, `<entity>_id` for FK refs,
  `payload` for opaque JSON. Adds `resolve_agent_id` helper with one-release deprecation
  window and 5 new unit tests.

## [0.3.11](https://github.com/Roberdan/convergio/compare/convergio-v0.3.10...convergio-v0.3.11) (2026-05-04)


### Features

* **cli-pr:** extract cvg pr into convergio-cli-pr + add pr merge wrapper ([823d694](https://github.com/Roberdan/convergio/commit/823d69491252cae9dbd88a50466535bb58a97c5d))
* **cli-pr:** extract cvg pr into convergio-cli-pr + add pr merge wrapper ([ee25095](https://github.com/Roberdan/convergio/commit/ee25095e45b556f2f0eb81cb877760ae769d60c9))
* **cli:** cvg bus tail --follow + cvg bus list (consume P1.1 SSE) ([14f07d3](https://github.com/Roberdan/convergio/commit/14f07d3a4472fdac48e6880c4111991299a2d13e))
* **cli:** cvg bus tail --follow and cvg bus list consume sse from p1.1 ([e59e825](https://github.com/Roberdan/convergio/commit/e59e825d3802c992ce67149a3abe7a8e89ec7774))
* **cli:** cvg discover — peer + bus + plan one-shot snapshot ([0b18b6a](https://github.com/Roberdan/convergio/commit/0b18b6aa00f563165271bacc81c5a2fd4851c153))
* **cli:** cvg discover — peer + bus + plan one-shot snapshot ([3e957d0](https://github.com/Roberdan/convergio/commit/3e957d0076db44d6d5e89056120aa3c868f10889))
* **cli:** cvg validate --self-test ([c2a94b5](https://github.com/Roberdan/convergio/commit/c2a94b550c2183fd05116f138a2013487bfbebbf))
* **cli:** cvg validate --self-test ([c37c8ae](https://github.com/Roberdan/convergio/commit/c37c8ae7fe6703c08ff2f09e267a83abdee8f0ea))
* **coherence:** add cvg coherence close-post-hoc verifier ([a7e11e4](https://github.com/Roberdan/convergio/commit/a7e11e4f501ec68cbedcc607d98849ab725f887e))
* **coherence:** cvg coherence close-post-hoc verifier ([4159cd8](https://github.com/Roberdan/convergio/commit/4159cd803fd56094751cf11352ab8fa9f612caa0))
* **coherence:** cvg coherence fleet sub-verifier ([7900788](https://github.com/Roberdan/convergio/commit/79007885f8d6d3b4e294f5ec13e5c11cc1ffcb57))
* **coherence:** cvg coherence fleet sub-verifier ([96f4556](https://github.com/Roberdan/convergio/commit/96f45565b7080edaf410f429080a7eab6a1cf49a))
* **coherence:** cvg coherence handshake — 2-session e2e smoke test ([68ae8d6](https://github.com/Roberdan/convergio/commit/68ae8d65931c68080482636bac9d3c4c91c6c4f0))
* **coherence:** cvg coherence handshake — 2-session e2e smoke test ([505a98a](https://github.com/Roberdan/convergio/commit/505a98ab57dbcb4b3720afceda8fd98ec0698d08))
* **tui:** cvg dash 5th pane: live bus tail filtered by selected plan ([98dc46d](https://github.com/Roberdan/convergio/commit/98dc46de99b2911ac4ef732a7f0a689a293e1b20))
* **tui:** cvg dash 5th pane: live bus tail filtered by selected plan ([e8324fb](https://github.com/Roberdan/convergio/commit/e8324fbfc101b2b3a93ea6f663dc8b6e60922969))


### Refactoring

* **server:** extract e2e boot() helper to tests/common/mod.rs ([88cdb76](https://github.com/Roberdan/convergio/commit/88cdb769b9f0bb96a5f288236f043ccf2332f1b7))
* **server:** extract e2e boot() to tests/common/mod.rs ([01bfff8](https://github.com/Roberdan/convergio/commit/01bfff867237799b522ee46cb27dece8927f2efb))


### Documentation

* **durability:** ADR-0042 wave-sequence gate refactor (parallel_safe) ([5ecd620](https://github.com/Roberdan/convergio/commit/5ecd620b9ed6d684eca3ca6ce56ede770c307ffb))
* **durability:** ADR-0042 wave-sequence gate refactor (parallel_safe) ([1bb584f](https://github.com/Roberdan/convergio/commit/1bb584f74fdb5432bcf0ce19e9693d5343086490))
* **repo:** regenerate auto blocks for tui dep change ([8e8b11e](https://github.com/Roberdan/convergio/commit/8e8b11e9659e6c6a36fda023e73b0b1c13e859c4))

## [0.3.10](https://github.com/Roberdan/convergio/compare/convergio-v0.3.9...convergio-v0.3.10) (2026-05-04)


### Features

* **brand:** adopt brand kit, claim, and shared convergio-brand crate ([4e01720](https://github.com/Roberdan/convergio/commit/4e01720285eeead7619c24d26b2eb42099d398c4))
* **brand:** adopt brand kit, claim, and shared convergio-brand crate ([89d3a1f](https://github.com/Roberdan/convergio/commit/89d3a1f6c4cc9ca3f7c57ef3f41db47844d24d88))
* **brand:** solid-block 4-row wordmark replaces ansi shadow ([0056327](https://github.com/Roberdan/convergio/commit/005632725f90bcf23ce4ec4493057a38c7b724d6))
* **brand:** solid-block 4-row wordmark replaces ansi shadow ([52cf503](https://github.com/Roberdan/convergio/commit/52cf50301eb7b4c896ca48da32f99acd871bafa5))
* **cli-session:** cvg session register-and-poll + SessionStart hook + status telemetry block (redux) ([496047d](https://github.com/Roberdan/convergio/commit/496047d412a406a5ed112398a1f4f157c8b9a6a6))
* **cli-session:** cvg session register-and-poll plus session-start hook and status telemetry block ([45e22c1](https://github.com/Roberdan/convergio/commit/45e22c15c91bd9a084989903256a46a5da0aeccb))
* **cli:** add fleet http routes and cvg fleet subcommands (f2-6) ([df142f0](https://github.com/Roberdan/convergio/commit/df142f0397c7590e812205cd919a9bb39fe92280))
* **cli:** bootstrap register-and-poll in per-host adapter prompt.txt ([6c35914](https://github.com/Roberdan/convergio/commit/6c3591484d9b8ef9975e8098e77a07a47164d6b7))
* **cli:** bootstrap register-and-poll in per-host adapter prompt.txt ([390ace1](https://github.com/Roberdan/convergio/commit/390ace1d87025ae9c0ea549b3fa57abd4509c93c))
* **cli:** cvg agent list/show enrichment + retire-stale ([2e547ce](https://github.com/Roberdan/convergio/commit/2e547ce706a0f0f22fbcd2bba37780f453eb1498))
* **cli:** cvg agent list/show enrichment + retire-stale ([c08caa7](https://github.com/Roberdan/convergio/commit/c08caa7f2246d0b7e7b632b8c3b45cf2abe6d76d))
* **cli:** cvg agent spawn auto-register + heartbeat (closes [#176](https://github.com/Roberdan/convergio/issues/176)) ([9199bf3](https://github.com/Roberdan/convergio/commit/9199bf3d9909e9758f5d2553c9565723cd3bf3b1))
* **cli:** cvg agent spawn auto-register + heartbeat (closes [#176](https://github.com/Roberdan/convergio/issues/176)) ([fc3d147](https://github.com/Roberdan/convergio/commit/fc3d1473f925ac0bce3c61655d7eff4872c8e130))
* **cli:** cvg coherence routes — deterministic route-table verifier ([9a19021](https://github.com/Roberdan/convergio/commit/9a190213522e99d22ac63ccf68eaf1a9ed65281f))
* **cli:** cvg coherence routes — deterministic route-table verifier ([fa6abe4](https://github.com/Roberdan/convergio/commit/fa6abe45085d9deaf7499c2d42376a7bdfc42200))
* **cli:** cvg monitor + big pixel banner for cvg dash ([2ab3419](https://github.com/Roberdan/convergio/commit/2ab341961a7b6782d8e005f8f054dc21f8633d5d))
* **cli:** cvg monitor + big pixel banner for cvg dash header ([5e7f9ed](https://github.com/Roberdan/convergio/commit/5e7f9ed2e3fd150850441c2a05668d9d26258ba5))
* **cli:** cvg plan triage — surface stale pending/failed tasks ([66817d4](https://github.com/Roberdan/convergio/commit/66817d41c025bc299363c47ad4ffcac8274e244e))
* **cli:** cvg plan triage — surface stale pending/failed tasks ([9e6ae6b](https://github.com/Roberdan/convergio/commit/9e6ae6b0efd3f591de371278ed5bbd638328eb25))
* **cli:** cvg session pre-stop check 1 — plan-vs-merged-pr drift ([64d6ab7](https://github.com/Roberdan/convergio/commit/64d6ab74553d071eb9c55908426bcf18af00e9d0))
* **cli:** cvg session pre-stop check 1 — plan-vs-merged-PR drift ([d5d6337](https://github.com/Roberdan/convergio/commit/d5d633774bcadbfdd7aaa18c27a2cec3e70279fb))
* **cli:** release-notes on cvg update + wiki sync workflow ([19f0414](https://github.com/Roberdan/convergio/commit/19f041414beb0cd0842bd820097f85e311144a15))
* **cli:** release-notes on cvg update + wiki sync workflow ([db23bf7](https://github.com/Roberdan/convergio/commit/db23bf7c6b13ebae7c51b60ec2efcd0105cdf3c2))
* **coherence:** cvg coherence adrs — adr status vs implementation cross-check ([051fcfc](https://github.com/Roberdan/convergio/commit/051fcfc6da149ea3489109864ce072aa447297c9))
* **coherence:** cvg coherence adrs — adr status vs implementation cross-check ([b924cbb](https://github.com/Roberdan/convergio/commit/b924cbbdc4720a41eb13b9f5b3d8e5207dfcdcb0))
* **coherence:** cvg coherence agents — pr author multi-agent protocol audit ([571e133](https://github.com/Roberdan/convergio/commit/571e133dee7f199743aa2741d896c919a1fd4078))
* **coherence:** cvg coherence agents verifier ([c04da64](https://github.com/Roberdan/convergio/commit/c04da6426d1a1e8c6fe09a609c1d530321b0a503))
* **embed:** --alpha linear blend on cvg graph for-task (F2-14) ([153e7c7](https://github.com/Roberdan/convergio/commit/153e7c77872e0a1fa7928e913cadb5c37eb3fd3a))
* **embed:** 30-fixture curated golden set + F1 retrospective rerun (adr-0038 §15.7) ([4960a13](https://github.com/Roberdan/convergio/commit/4960a13acaa4b8e8bb0d8c11ee940ee07635331b))
* **embed:** 30-fixture curated golden set + F1 retrospective rerun (adr-0038 §15.7) ([d79b3d4](https://github.com/Roberdan/convergio/commit/d79b3d44565c338383120f894deb70d5b14ec8c6))
* **embed:** add linear_blend_fuse + --alpha on cvg graph for-task ([07ebeda](https://github.com/Roberdan/convergio/commit/07ebeda67dee6b69f353416e2265baf9a6415d01))
* **embed:** convergio-embed crate foundation (adr-0035 f1-α) ([5e8d6c0](https://github.com/Roberdan/convergio/commit/5e8d6c0e46c39640748a0d5dc4da948752cd77b6))
* **embed:** convergio-embed crate foundation (adr-0038 f1-α) ([63e22ab](https://github.com/Roberdan/convergio/commit/63e22ab3a3e28f10f87476cb669fd1d073b44066))
* **embed:** hybrid retrieval (rrf) + cvg graph for-task --semantic — adr-0038 f1-ε ([ec8e872](https://github.com/Roberdan/convergio/commit/ec8e8725626340500761b42b7c1aed48c0804fb1))
* **embed:** hybrid retrieval (rrf) + cvg graph for-task --semantic (adr-0038 f1-ε) ([209067a](https://github.com/Roberdan/convergio/commit/209067af942e23ec066350bcf85889585392d982))
* **embed:** real model via fastembed-rs — adr-0038 f1-β ([5d8a938](https://github.com/Roberdan/convergio/commit/5d8a9389f948ecdf1d8a45fe117cb25302190329))
* **embed:** real model via fastembed-rs (multilingual-e5-small) ([7b6eea7](https://github.com/Roberdan/convergio/commit/7b6eea743246d0affd2c4bfd78d7f47c10dc785c))
* **embed:** recall benchmark + F1 retrospective — adr-0038 f1-ζ ([3d63c88](https://github.com/Roberdan/convergio/commit/3d63c88d00cbc1584d01e2d611589320cf19f686))
* **embed:** recall benchmark + f1 retrospective (adr-0038 f1-ζ) ([2578399](https://github.com/Roberdan/convergio/commit/2578399a02d2acb13abc7e5cf45c2e2088a28f29))
* **embed:** wire embedder + ingest + warm/build/for-task — adr-0038 f1-γ ([a82e7c3](https://github.com/Roberdan/convergio/commit/a82e7c3ee829fec6f370caea1c617f02cded05bb))
* **embed:** wire embedder + ingest + warm/build/for-task (adr-0038 f1-γ) ([8de7eb7](https://github.com/Roberdan/convergio/commit/8de7eb777e2903d8ab7aaf67ec576ec17cbaa6d4))
* **executor:** per-task runner_kind/profile/max_budget_usd (adr-0034) ([9090bbf](https://github.com/Roberdan/convergio/commit/9090bbfcf112c6480e235cfd4b4fcc4ce98c5dca))
* **executor:** per-task runner_kind/profile/max_budget_usd (ADR-0034) ([de4579e](https://github.com/Roberdan/convergio/commit/de4579e6b445c7257837bc864cbd5cc422d8b716))
* **fleet:** add fleet-scope recall bench and cross-repo fixtures (f2-12) ([e61a7c5](https://github.com/Roberdan/convergio/commit/e61a7c5a172017869479eb35951b1c3a0acf24e3))
* **fleet:** add post /v1/fleet/build with parse+embed orchestration (f2-7) ([83df031](https://github.com/Roberdan/convergio/commit/83df031d84c39f6fa9921c9ec7c37baf501143c7))
* **fleet:** bootstrap convergio-fleet skeleton (adr-0038 f2-4) ([f033e09](https://github.com/Roberdan/convergio/commit/f033e096d416460d969493babc630ee1fff8763b))
* **fleet:** bootstrap crates/convergio-fleet skeleton crate ([780ae1f](https://github.com/Roberdan/convergio/commit/780ae1f0f42c5d3b66a0692262d2178d9c6a7223))
* **fleet:** cross-repo cluster detection via union-find (adr-0038 f2-9) ([ddd02e8](https://github.com/Roberdan/convergio/commit/ddd02e8b84b43bce2a1a05db2cea62406388dcbe))
* **fleet:** cross-repo cluster detection via union-find (f2-9) ([5833a26](https://github.com/Roberdan/convergio/commit/5833a262e98d7631fbfbbd1e4de60274661726ca))
* **fleet:** cross-repo duplicate pairs via cosine threshold (f2-10) ([ae06e55](https://github.com/Roberdan/convergio/commit/ae06e55c9a1acf1774cbb4385da83be893d3c570))
* **fleet:** cross-repo similarity batch with cosine+shape match (adr-0038 f2-8) ([35d5180](https://github.com/Roberdan/convergio/commit/35d5180dea092585943ac0a7c9a5fe2a58f1886e))
* **fleet:** cross-repo similarity batch with cosine+shape match (f2-8) ([315c395](https://github.com/Roberdan/convergio/commit/315c39589883c73e142ee12535ee12a7a524b7e3))
* **fleet:** cvg fleet add/ls/disable/enable + /v1/fleet/repos (adr-0038 f2-6) ([85e2d32](https://github.com/Roberdan/convergio/commit/85e2d32d221476267c7a4dd102ac6a378aec037b))
* **fleet:** cvg fleet build orchestrator + similarity edges (adr-0038 f2-7) ([79982e1](https://github.com/Roberdan/convergio/commit/79982e1174bd2b4fa53d73ce9d6e5c7dc6e7b651))
* **fleet:** cvg fleet duplicates with diff preview (adr-0038 f2-10) ([3c3159e](https://github.com/Roberdan/convergio/commit/3c3159e9c2ded73ca338b5bd27716c76c73521f9))
* **fleet:** fleet-scope recall bench + 30 cross-repo fixtures (F2-12) ([17a55fb](https://github.com/Roberdan/convergio/commit/17a55fb01155fcb8c966339fab50200674b6bcf8))
* **graph:** add repo dimension to graph_nodes ([abcde62](https://github.com/Roberdan/convergio/commit/abcde620ce1018403d06b48aac027f61c9f819ec))
* **graph:** add repo dimension to graph_nodes (adr-0038 f2-5) ([7cc4c2c](https://github.com/Roberdan/convergio/commit/7cc4c2c572cc421a196d0f57522e1d8c5e811dfd))
* **parse-multi:** add Python parser integration tests ([98dcbf5](https://github.com/Roberdan/convergio/commit/98dcbf5814d0f66cc71a4923d68bccd498296a46))
* **parse-multi:** add Python parser py.rs ([1876014](https://github.com/Roberdan/convergio/commit/1876014722889bfcdec5787a9d495eb93b9fd43f))
* **parse-multi:** add Python test fixture sample.py ([8f293f3](https://github.com/Roberdan/convergio/commit/8f293f3aa59a11bf6f336d2b576b3c12484c5c4e))
* **parse-multi:** bootstrap convergio-parse-multi crate (adr-0038 f2-1) ([443f7b0](https://github.com/Roberdan/convergio/commit/443f7b08111ebe95ff7b4c30b9dae02112d6c351))
* **parse-multi:** bootstrap convergio-parse-multi crate (adr-0038 f2-1) ([a29075e](https://github.com/Roberdan/convergio/commit/a29075e87d194ab7308d3646bb16f7aba18f73fa))
* **parse-multi:** export parse_py from lib.rs ([36a6aad](https://github.com/Roberdan/convergio/commit/36a6aad9ae4e731f267da70bcf66ea36f76b64f9))
* **parse-multi:** Python parser produces (Vec&lt;Node&gt;, Vec&lt;Edge&gt;) ([9aaa89e](https://github.com/Roberdan/convergio/commit/9aaa89e16c8f67d528150b52d0ce33952a1761b0))
* **parse-multi:** TypeScript parser produces (Vec&lt;Node&gt;, Vec&lt;Edge&gt;) ([a73505c](https://github.com/Roberdan/convergio/commit/a73505c8d1ff083a785532ec2b40d42fb76b89ef))
* **parse-multi:** TypeScript parser produces (Vec&lt;Node&gt;, Vec&lt;Edge&gt;) (adr-0038 f2-2) ([e077619](https://github.com/Roberdan/convergio/commit/e0776196cdbfcad705dfce7c1c62519be1776f79))
* **planner:** opus-backed planner replaces line-split heuristic (adr-0036) ([e6a57b7](https://github.com/Roberdan/convergio/commit/e6a57b70ad922719c6a86cc6391ee3e67b5b41bb))
* **planner:** opus-backed planner replaces line-split heuristic (ADR-0036) ([89a0665](https://github.com/Roberdan/convergio/commit/89a0665a75cb5968d4fd0837a37fca37c469cfa8))
* **repo:** lefthook pre-push gate on docs regenerate + index freshness ([c238ba3](https://github.com/Roberdan/convergio/commit/c238ba3bb162b3949c0425dfc51d32d5d7593fd4))
* **repo:** lefthook pre-push gate on docs regenerate + index freshness ([c345099](https://github.com/Roberdan/convergio/commit/c345099468c8213f91ccf9fb7ce390879a100e7e))
* **runner:** permission profiles replace --dangerously-skip-permissions ([1dd49b2](https://github.com/Roberdan/convergio/commit/1dd49b210f6de653e8b250f0df5f7bf714640679))
* **runner:** permission profiles replace dangerously-skip-permissions ([20b769e](https://github.com/Roberdan/convergio/commit/20b769e9ecea9b966590158654c2fd3f62338ada))
* **runner:** toml registry for custom vendors (adr-0035) ([3a44764](https://github.com/Roberdan/convergio/commit/3a44764242616b86eca15b41039ef008c9b73592))
* **runner:** TOML registry for custom vendors (ADR-0035) ([dc03cfd](https://github.com/Roberdan/convergio/commit/dc03cfd27dd378dea211c90d7fc808a47af26a0c))
* **server:** add f2-13 cross-repo measurement e2e test ([34eb64a](https://github.com/Roberdan/convergio/commit/34eb64a214bbc986452bef6d4ca25c406babd165))
* **server:** F2-13 cross-repo measurement e2e test ([27f22d3](https://github.com/Roberdan/convergio/commit/27f22d3bfa78e734be83819b1b06559b75c76212))
* **server:** SSE endpoints for audit + plan messages stream ([8bd11a2](https://github.com/Roberdan/convergio/commit/8bd11a25b0f51852a45418d8ee8c4c5f981bb410))
* **server:** SSE endpoints for audit + plan messages stream (P1.1) ([6799cc6](https://github.com/Roberdan/convergio/commit/6799cc640e9a203805c93d18b6326da080bfb100))
* **skills:** /cvg-spawn skill + agent registry kind=subagent ([1fb246d](https://github.com/Roberdan/convergio/commit/1fb246da2551c40ce8c087ed8a7794491ee4811b))
* **skills:** /cvg-spawn skill + agent registry kind=subagent ([a6cd489](https://github.com/Roberdan/convergio/commit/a6cd489c2b0cae17dd4bf20bae821a50c3e2b0d2))
* **tui:** expand cvg dash history drilldown ([8cbc29e](https://github.com/Roberdan/convergio/commit/8cbc29ea37f72a9767161d46c0a8bc231105e022))
* **tui:** expand dash history drilldown ([d49da49](https://github.com/Roberdan/convergio/commit/d49da49cb9b83c8f5cf8e4cc353e0c92051e5546))
* **tui:** retire 4-row big banner, keep 2-row max ([3fcfbe7](https://github.com/Roberdan/convergio/commit/3fcfbe7dafc034d2a6b488dbc6a656aece710634))
* **tui:** retire 4-row big banner, keep 2-row max ([b40d295](https://github.com/Roberdan/convergio/commit/b40d295d3ec840656c4f21a432a8c5a20b14510f))


### Bug Fixes

* **brand:** serialize theme env tests ([e109435](https://github.com/Roberdan/convergio/commit/e109435a41b2ea331b93ba719e555a6415988248))
* **cli:** backfill repo_path into existing configs on cvg setup init ([d8351c3](https://github.com/Roberdan/convergio/commit/d8351c32d4c6e2e44bd25ea726879646d8880412))
* **cli:** backfill repo_path into existing configs on cvg setup init ([154c080](https://github.com/Roberdan/convergio/commit/154c0809ed7cfeec2ee919f78bddee516c49c3ba))
* **runner:** stream-json + permission-bypass + live stdout ([24f59a6](https://github.com/Roberdan/convergio/commit/24f59a696cd808277326707146dccb759922dee7))
* **runner:** stream-json + permission-bypass + live stdout ([d426a10](https://github.com/Roberdan/convergio/commit/d426a101109c935bf2fb5e68d37891ba32db7bb6))


### Refactoring

* **coherence:** extract cvg coherence suite into convergio-coherence crate ([1ef57e4](https://github.com/Roberdan/convergio/commit/1ef57e42a9ab883e97dc71336dc319d744782975))
* **coherence:** extract cvg coherence suite into convergio-coherence crate ([ff59924](https://github.com/Roberdan/convergio/commit/ff5992454eef40f77b0a63b995a50fb5a022dcb7))
* **session:** extract cvg session suite into convergio-cli-session crate ([d0c7ff1](https://github.com/Roberdan/convergio/commit/d0c7ff179830fb1e1d57e93ed0d16d50aa23c668))
* **session:** extract cvg session suite into convergio-cli-session crate ([573ff1f](https://github.com/Roberdan/convergio/commit/573ff1f86ff24c7e1c144c2d86566616a165bdb2))


### Documentation

* F2 retrospective ADR + plan close-out (F2-15) ([3efa412](https://github.com/Roberdan/convergio/commit/3efa41278dcda5819a2f76dfe19e4825ca839695))
* **fleet:** regenerate auto blocks after cluster patterns (f2-9) ([c9e9f2b](https://github.com/Roberdan/convergio/commit/c9e9f2bd9a03e9943a3c87840093fd4387becff7))
* **fleet:** regenerate auto blocks after duplicates ([0659e4a](https://github.com/Roberdan/convergio/commit/0659e4a9bfaab7341958531a8c3f5e7b47973b22))
* **fleet:** regenerate auto blocks after fleet build orchestrator ([dbde37a](https://github.com/Roberdan/convergio/commit/dbde37a29ee80b0aaabf1b9ae2fdf891af36dbfb))
* **fleet:** regenerate auto blocks after fleet cli additions ([cfbd8b1](https://github.com/Roberdan/convergio/commit/cfbd8b1c1167d99eb61c6fcf7229b56836f14e29))
* **fleet:** regenerate auto blocks after fleet crate ([96491a6](https://github.com/Roberdan/convergio/commit/96491a6eca47377c380ac0feceff731ac19c315f))
* **fleet:** regenerate auto blocks after patterns ([878f91c](https://github.com/Roberdan/convergio/commit/878f91cafbe17e6f458b82e0721ff0eaa3966ac0))
* **fleet:** regenerate auto blocks after similarity batch ([57d7a15](https://github.com/Roberdan/convergio/commit/57d7a153ef1fbb0c973988385b6dcfaf35d9e88d))
* **graph:** regenerate auto blocks after repo dimension migration ([49fb00f](https://github.com/Roberdan/convergio/commit/49fb00f189d1d360b2e490c8b7d1eb163ea58458))
* **parse-multi:** regenerate auto blocks after python parser ([9e076b7](https://github.com/Roberdan/convergio/commit/9e076b7387a7edd90a7b4fbc8afaeb0c4f4e0703))
* **parse-multi:** regenerate auto blocks after ts parser additions ([dee5eb9](https://github.com/Roberdan/convergio/commit/dee5eb91a97179457f35d57f4e3fad3bcbac0ca3))
* **repo:** coherence + token-efficiency pass on docs/, root, ignore lists ([004c76a](https://github.com/Roberdan/convergio/commit/004c76a75bec50fa520c9e18c2ee0c97a282563f))
* **repo:** coherence + token-efficiency pass on docs/, root, ignore lists ([417aafd](https://github.com/Roberdan/convergio/commit/417aafd7f132f0ab76759403d8a94b7421d6cd3e))
* **repo:** F2 retrospective ADR + plan close-out (f2-15) ([8f7a0c9](https://github.com/Roberdan/convergio/commit/8f7a0c9a9bbefb3de2b01bd4706a2bfe069d9fb8))
* **repo:** flip 3 adrs to accepted after audit + annotate the rest ([46d52c9](https://github.com/Roberdan/convergio/commit/46d52c9d417cd80310eff4bcb68108a1b81cac97))
* **repo:** flip 3 ADRs to accepted after audit + annotate the rest ([9a36f72](https://github.com/Roberdan/convergio/commit/9a36f722f9c43679da18b3e90b324d64bef934fc))
* **repo:** position convergio vs gstack/gbrain, surface adr-0034..0036 ([7e9a9de](https://github.com/Roberdan/convergio/commit/7e9a9de3a26caef090f3f05c4d5c0261a054fd5c))
* **repo:** position Convergio vs gstack/gbrain, surface ADR-0034..0036 ([c7ae226](https://github.com/Roberdan/convergio/commit/c7ae226a9be4d17c7c334d1855fc771bd33c2279))
* **repo:** re-sync index after crate stats regen ([b174a6d](https://github.com/Roberdan/convergio/commit/b174a6d248d54e635cb827dfc35443e4d11876e3))
* **repo:** refresh docs index ([2b3f5c6](https://github.com/Roberdan/convergio/commit/2b3f5c6a379f83c6eab1723b0a75f54472db6a76))
* **repo:** regenerate auto blocks ([d3e7f06](https://github.com/Roberdan/convergio/commit/d3e7f06aadaf47c6268a0319bb6d3b5eb01be6c8))
* **repo:** regenerate auto blocks + index for parse-multi crate ([d326a8c](https://github.com/Roberdan/convergio/commit/d326a8cb712ae2ce4c9df4185abc823aa9a00f6d))
* **repo:** regenerate auto blocks after brand kit additions ([a4739e5](https://github.com/Roberdan/convergio/commit/a4739e5fc134416cbee9f111d235e6a98cd480e3))
* **repo:** regenerate auto blocks after f2-11 ([9630b89](https://github.com/Roberdan/convergio/commit/9630b89552518f91e6a3a4db8d49145da11d972a))
* **repo:** regenerate auto blocks after f2-12 ([245b212](https://github.com/Roberdan/convergio/commit/245b21275f1f108da4a8ab8de079ac14ccfd3da3))
* **repo:** regenerate auto blocks after f2-14 ([bda96a5](https://github.com/Roberdan/convergio/commit/bda96a53331393d6628f6cf9d88c9dc09c7aaa6b))
* **repo:** regenerate auto blocks after merge with main ([9a55778](https://github.com/Roberdan/convergio/commit/9a55778bcc56090da22bdb74b3c17133e7ac3230))
* **repo:** regenerate auto blocks for adr-0035 f1-α ([007bf3b](https://github.com/Roberdan/convergio/commit/007bf3b343bdd92e43fd5c26508070e61b5a1244))
* **repo:** regenerate auto blocks for setup_repo_path ([4b4d7db](https://github.com/Roberdan/convergio/commit/4b4d7db0b0527c07e7f3863848acb9206ebd5287))
* **repo:** regenerate cli auto blocks after embed.rs trim ([2c4fd9f](https://github.com/Roberdan/convergio/commit/2c4fd9f5bee7970ba1b6cec389be6d510c6690bb))
* **repo:** regenerate crate stats blocks for adr-0034 ([e8588e4](https://github.com/Roberdan/convergio/commit/e8588e44ea5d28ba79394263e19a42124458b492))
* **repo:** regenerate docs index ([794eccf](https://github.com/Roberdan/convergio/commit/794eccfda4b5cac6f069ebb3da3e0977ca2503ba))
* **repo:** regenerate docs index after pixel banner additions ([aeb9699](https://github.com/Roberdan/convergio/commit/aeb9699d572e00820ba76e3997f28247ed672fa8))
* **repo:** regenerate docs index after setup repo path fix ([5370183](https://github.com/Roberdan/convergio/commit/537018311a801141e87cef1c8d4f21eeccfd7e0f))
* **repo:** regenerate docs index for adr-0035 entries ([499af08](https://github.com/Roberdan/convergio/commit/499af08caa8688ab83dfeffedbb00fe6192f8ef5))
* **repo:** regenerate index after adr-0038 §15.7 expansion ([631d0b7](https://github.com/Roberdan/convergio/commit/631d0b7c72defaab14120c3668ede06f6bc212e0))
* **repo:** regenerate index after agents auto blocks update ([79057bf](https://github.com/Roberdan/convergio/commit/79057bf78dbd1a29130b8a04b540fab2ca556298))
* **repo:** regenerate index after brand kit additions ([751ab97](https://github.com/Roberdan/convergio/commit/751ab970c87a53500d8386e347b337704810cc85))
* **repo:** regenerate index for f2-13 ([e6ddab5](https://github.com/Roberdan/convergio/commit/e6ddab53b0fb520bcc90419a753a26313dbcd9e5))
* **repo:** regenerate index with adr-0034 entry ([5a3d321](https://github.com/Roberdan/convergio/commit/5a3d3217a5d5142f8b0176c891b1bb8d6d572751))
* **repo:** renumber fleet adr 0035→0038 to clear collision ([08e241f](https://github.com/Roberdan/convergio/commit/08e241f972cc0975a52b06b378b3b7459f6a562c))
* **repo:** seed adr-0039 doc-coherence sweep three-layer plan ([48420f0](https://github.com/Roberdan/convergio/commit/48420f08fb2ba74b01b029189b77879ef710fd78))
* **repo:** seed adr-0039 doc-coherence sweep three-layer plan ([a78ea8f](https://github.com/Roberdan/convergio/commit/a78ea8f1fd84b92760592b5b4eccede3aabfe088))
* **repo:** seed fleet retrieval foundation (adr-0035) ([006a167](https://github.com/Roberdan/convergio/commit/006a16724e0cb4de6cee9c781fb35d521ebf312e))
* **repo:** seed fleet retrieval foundation (adr-0035) ([1220a81](https://github.com/Roberdan/convergio/commit/1220a81c56b55de0aa0ac2d0e050e5c3011ec512))
* **repo:** seed fleet retrieval foundation (adr-0038) ([0bc2b4b](https://github.com/Roberdan/convergio/commit/0bc2b4b70c160e105ec8f3a6777a90a1ae8d6040))

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
