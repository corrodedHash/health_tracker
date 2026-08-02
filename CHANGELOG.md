# Changelog

## [0.6.0](https://github.com/corrodedHash/health_tracker/compare/health-tracker-v0.5.0...health-tracker-v0.6.0) (2026-08-02)


### Features

* **frontend:** add API token management UI ([#54](https://github.com/corrodedHash/health_tracker/issues/54)) ([004d778](https://github.com/corrodedHash/health_tracker/commit/004d77839fb85f833604651f326780940162426e))

## [0.5.0](https://github.com/corrodedHash/health_tracker/compare/health-tracker-v0.4.0...health-tracker-v0.5.0) (2026-08-01)


### Features

* **db:** embed migrations in the binary ([#53](https://github.com/corrodedHash/health_tracker/issues/53)) ([3f65f66](https://github.com/corrodedHash/health_tracker/commit/3f65f6619352b69e9fa76bba2fe45556a4b6bf3c))


### Bug Fixes

* **bot:** use HEALTH__ separator for env config ([#51](https://github.com/corrodedHash/health_tracker/issues/51)) ([9abd946](https://github.com/corrodedHash/health_tracker/commit/9abd946e7dd966a23e2f35309c36f38eaaf15068))

## [0.4.0](https://github.com/corrodedHash/health_tracker/compare/health-tracker-v0.3.1...health-tracker-v0.4.0) (2026-07-26)


### Features

* add Dex OIDC provider in Docker for dev testing ([#42](https://github.com/corrodedHash/health_tracker/issues/42)) ([1112d76](https://github.com/corrodedHash/health_tracker/commit/1112d7661d91c2b28100c30c3cf1943a8651ff00))
* add Synapse E2E test setup script and local config support ([#40](https://github.com/corrodedHash/health_tracker/issues/40)) ([1d3450e](https://github.com/corrodedHash/health_tracker/commit/1d3450e31e0d87b00d3a9e741570591a35663286))
* derive debug database name from branch for worktree isolation ([#41](https://github.com/corrodedHash/health_tracker/issues/41)) ([3f93766](https://github.com/corrodedHash/health_tracker/commit/3f937664380a6610b3781b6d92db8a377bb82980))
* load web config from TOML files too ([#48](https://github.com/corrodedHash/health_tracker/issues/48)) ([4674cfc](https://github.com/corrodedHash/health_tracker/commit/4674cfcb2f1801e72356e5e3bbc2219fbc434ed4)), closes [#45](https://github.com/corrodedHash/health_tracker/issues/45)
* scope frontend mise tasks to frontend/ via monorepo_root ([#43](https://github.com/corrodedHash/health_tracker/issues/43)) ([380c8f7](https://github.com/corrodedHash/health_tracker/commit/380c8f7d7a425e630776bca508f324fde8f9ccb6))


### Bug Fixes

* mise monoroot repo ([#47](https://github.com/corrodedHash/health_tracker/issues/47)) ([8110981](https://github.com/corrodedHash/health_tracker/commit/811098143717169c64733e6be746d941e499e366))
* move OIDC auth routes under /api/auth to fix 404 behind reverse proxy ([#39](https://github.com/corrodedHash/health_tracker/issues/39)) ([58c0422](https://github.com/corrodedHash/health_tracker/commit/58c04225b618edf267d59b25d6765ecb9976d8f6))
* specify pnpm minor version ([#37](https://github.com/corrodedHash/health_tracker/issues/37)) ([a855cad](https://github.com/corrodedHash/health_tracker/commit/a855caddfc731c4de317019483a8e6fe7a4df15d))

## [0.3.1](https://github.com/corrodedHash/health_tracker/compare/health-tracker-v0.3.0...health-tracker-v0.3.1) (2026-07-19)


### Bug Fixes

* build workspace members and drop non-linux targets in release workflow ([#35](https://github.com/corrodedHash/health_tracker/issues/35)) ([3f108dd](https://github.com/corrodedHash/health_tracker/commit/3f108ddbddb78403bf9e7f6d76e11fbc4bf2bdce))

## [0.3.0](https://github.com/corrodedHash/health_tracker/compare/health-tracker-v0.2.1...health-tracker-v0.3.0) (2026-07-19)


### Features

* build and upload frontend artifact in release workflow ([#33](https://github.com/corrodedHash/health_tracker/issues/33)) ([75f3dcb](https://github.com/corrodedHash/health_tracker/commit/75f3dcb3bcb979814f1804fd53de3ad30d5fc40b))


### Bug Fixes

* pass --repo flag to gh release upload to fix upload job ([#31](https://github.com/corrodedHash/health_tracker/issues/31)) ([1ade56f](https://github.com/corrodedHash/health_tracker/commit/1ade56fbf0264673380be5e5bbf8348611dd74a5))

## [0.2.1](https://github.com/corrodedHash/health_tracker/compare/health-tracker-v0.2.0...health-tracker-v0.2.1) (2026-07-18)


### Bug Fixes

* cargo lock file ([#29](https://github.com/corrodedHash/health_tracker/issues/29)) ([2b2d25d](https://github.com/corrodedHash/health_tracker/commit/2b2d25de4eac68430b8f002cf953856fc66058c5))
* remove cargo-workspace plugin (conflicts with Rust strategy's native workspace handling) ([#28](https://github.com/corrodedHash/health_tracker/issues/28)) ([51d9dac](https://github.com/corrodedHash/health_tracker/commit/51d9dac226c17f5f1e1be3684902468678b29f06))

## [0.2.0](https://github.com/corrodedHash/health_tracker/compare/health-tracker-v0.1.0...health-tracker-v0.2.0) (2026-07-18)


### Features

* add dev_auto_login mode bypassing OIDC ([7f849c0](https://github.com/corrodedHash/health_tracker/commit/7f849c0436f8d15e6de9ed22935e3452d710d080))
* add ensure-debug-database mise task and wire into run-web ([#4](https://github.com/corrodedHash/health_tracker/issues/4)) ([37fdbb3](https://github.com/corrodedHash/health_tracker/commit/37fdbb3b97889a9d9b43ebda89bce461a1243585))
* add OpenAPI spec generation via utoipa + fix duration_secs field mismatch ([6b76c4b](https://github.com/corrodedHash/health_tracker/commit/6b76c4b541c3f0afb705beab2a63f22c28620bff))
* add stopwatch widget with localStorage persistence ([#18](https://github.com/corrodedHash/health_tracker/issues/18)) ([303cf5d](https://github.com/corrodedHash/health_tracker/commit/303cf5d7f5d0348a5e12b155aaaa9a04faef3d5f))
* bot + frontend ([#1](https://github.com/corrodedHash/health_tracker/issues/1)) ([ff3b15f](https://github.com/corrodedHash/health_tracker/commit/ff3b15ff07a08162c58f26206f04917a801e4989))
* custom exercises ([#16](https://github.com/corrodedHash/health_tracker/issues/16)) ([6e141e6](https://github.com/corrodedHash/health_tracker/commit/6e141e6d701850656551dfb5835e876bed14db83))
* **dashboard:** graphs tab with running pace, distance, and training heatmap ([#17](https://github.com/corrodedHash/health_tracker/issues/17)) ([07f8732](https://github.com/corrodedHash/health_tracker/commit/07f873288998bbbe534c557a20ed76b7970b54f6))
* **db:** add migrations 0001-0008 and SqlxRepository impl (Phase 1 5.1-5.9) ([2e893e4](https://github.com/corrodedHash/health_tracker/commit/2e893e46c4295d4a379b2c2cc6e03cca3ee2b52b))
* exercise specific data in frontend ([#7](https://github.com/corrodedHash/health_tracker/issues/7)) ([af399ad](https://github.com/corrodedHash/health_tracker/commit/af399ad6c996ee575e4d839b53ff40fa5116f31b))
* isolate sqlx-prepare per branch to avoid worktree conflicts ([#15](https://github.com/corrodedHash/health_tracker/issues/15)) ([e39dee5](https://github.com/corrodedHash/health_tracker/commit/e39dee5c8aae6951868c94f00a318a241bc582ca))
* paginate sessions (closes [#6](https://github.com/corrodedHash/health_tracker/issues/6)) ([#11](https://github.com/corrodedHash/health_tracker/issues/11)) ([5da7e99](https://github.com/corrodedHash/health_tracker/commit/5da7e99f6aae6d66045cf4f0a71fcf5729c586a1))
* set up release-please for automated releases ([#21](https://github.com/corrodedHash/health_tracker/issues/21)) ([756a1e0](https://github.com/corrodedHash/health_tracker/commit/756a1e00da1b2c4277e814718fdb2582bc3b7225))


### Bug Fixes

* add [package] to root Cargo.toml for release-please plugin compat ([#25](https://github.com/corrodedHash/health_tracker/issues/25)) ([ced9d95](https://github.com/corrodedHash/health_tracker/commit/ced9d95ddf2904ccef436c97a35254e82c19a5e4))
* add cargo-workspace plugin to release-please-config ([#22](https://github.com/corrodedHash/health_tracker/issues/22)) ([946161c](https://github.com/corrodedHash/health_tracker/commit/946161c01db1d93d86df8882be925c0929d0782c))
* align frontend auth route with backend, add dev tasks to mise ([ec30b79](https://github.com/corrodedHash/health_tracker/commit/ec30b795449f7f03635768eefb7c32156d51ec10))
* clear clippy warnings flagged by mise lint ([e575854](https://github.com/corrodedHash/health_tracker/commit/e575854fde7229a2e58ef9100e54c4a8dcdc2aaf))
* formatting and pnpm version fixing ([05534d1](https://github.com/corrodedHash/health_tracker/commit/05534d156863527e04ec1be62810f7a9f8839d6f))
* frontend session reset - auth loading state, conditional rendering, promise chain ([#10](https://github.com/corrodedHash/health_tracker/issues/10)) ([6efddc1](https://github.com/corrodedHash/health_tracker/commit/6efddc1afe7b283226d8dd091b815ac7a3c16468))
* release please ([#26](https://github.com/corrodedHash/health_tracker/issues/26)) ([0431f35](https://github.com/corrodedHash/health_tracker/commit/0431f3511d6a6a471ce6c4cd5ac28ed0e862eb53))
* replace version.workspace with literal version strings for release-please compat ([#24](https://github.com/corrodedHash/health_tracker/issues/24)) ([36dd8d4](https://github.com/corrodedHash/health_tracker/commit/36dd8d4d3dde42b86a7d4526757c65baf795b449))
* test timeout and formatting ([63d17fb](https://github.com/corrodedHash/health_tracker/commit/63d17fb2fef27f58b4473f2aedb7d46a0b76447d))
* use 302 FOUND instead of 200 OK for login redirect ([cfa1c30](https://github.com/corrodedHash/health_tracker/commit/cfa1c30e669de7475fb7cd6d5d353c51304317fa))
