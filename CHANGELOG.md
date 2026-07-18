# Changelog

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
