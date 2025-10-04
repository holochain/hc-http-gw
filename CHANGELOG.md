# Changelog

All notable changes to this project will be documented in this file.

This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## \[[0.3.0](https://github.com/holochain/hc-http-gw/compare/v0.2.0...v0.3.0)\] - 2025-10-04

### Features

- Upgrade to Holochain 0.6, currently using the 0.6.0-dev.27 version (#62) by @ThetaSinner in [#62](https://github.com/holochain/hc-http-gw/pull/62)

### Bug Fixes

- Feature logic with upgrade to 0.5.5 by @ThetaSinner in [#59](https://github.com/holochain/hc-http-gw/pull/59)

### Miscellaneous Tasks

- Update Cargo.lock file by @cdunster
- Update flake.lock file (#58) by @holochain-release-automation2 in [#58](https://github.com/holochain/hc-http-gw/pull/58)

### First-time Contributors

- @holochain-release-automation2 made their first contribution in [#58](https://github.com/holochain/hc-http-gw/pull/58)
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2025-07-09

### Added

- Add missing CI devShell by @cdunster in [#56](https://github.com/holochain/hc-http-gw/pull/56)
- Add hc package to devShell for packaging fixtures by @cdunster
- Add rust toolchain as watched file for direnv by @cdunster
- Add cron job to update the Cargo lockfile by @cdunster in [#48](https://github.com/holochain/hc-http-gw/pull/48)
- Add cron job to update the flake lockfile by @cdunster
- Add CHANGELOG for existing release by @cdunster in [#47](https://github.com/holochain/hc-http-gw/pull/47)
- Add workflow to publish a release created by the prepare workflow by @cdunster
- Add workflow to prepare a release by @cdunster
- Add LICENSE (#44) by @ThetaSinner in [#44](https://github.com/holochain/hc-http-gw/pull/44)
- Add load test (#43) by @c12i in [#43](https://github.com/holochain/hc-http-gw/pull/43)

### Changed

- Put CI workflow run command onto a single line by @cdunster
- Cargo update by @cdunster in [#55](https://github.com/holochain/hc-http-gw/pull/55)
- Update rustc to v1.85.0 by @cdunster
- Only restrict cargo dependencies to highest version part by @cdunster
- Update test fixtures to holochain v0.5 by @cdunster
- Update to holochain v0.5 by @cdunster
- Update holonix to use `main-0.5` branch by @cdunster
- Update flake.lock file by @cdunster in [#52](https://github.com/holochain/hc-http-gw/pull/52)
- Update rustc to 1.84.0 by @cdunster in [#51](https://github.com/holochain/hc-http-gw/pull/51)
- Update serde_json to use Holochain fork by @cdunster in [#49](https://github.com/holochain/hc-http-gw/pull/49)
- Run changelog preview job in test workflow by @cdunster
- Don't run test workflow on pushes to main by @cdunster
- Spec updates (#41) by @ThetaSinner in [#41](https://github.com/holochain/hc-http-gw/pull/41)
- Move spec to repo (#34) by @c12i in [#34](https://github.com/holochain/hc-http-gw/pull/34)

## [0.1.0] - 2025-03-12

### Added

- Add steps in README to test and run locally by @cdunster in [#35](https://github.com/holochain/hc-http-gw/pull/35)
- Add integration tests by @ThetaSinner in [#32](https://github.com/holochain/hc-http-gw/pull/32)
- Add extra tests and Go package by @cdunster in [#30](https://github.com/holochain/hc-http-gw/pull/30)
- Add app websocket management by @ThetaSinner in [#17](https://github.com/holochain/hc-http-gw/pull/17)
- Add validation tests to zome call route by @jost-s
- Add fns to transcode between ExternIO and JSON by @jost-s
- Add Nix devShell with direnv support by @cdunster in [#16](https://github.com/holochain/hc-http-gw/pull/16)

### Changed

- Update version for publish by @ThetaSinner in [#40](https://github.com/holochain/hc-http-gw/pull/40)
- Deserialize binary to JSON by @ThetaSinner in [#39](https://github.com/holochain/hc-http-gw/pull/39)
- Update response by @ThetaSinner in [#37](https://github.com/holochain/hc-http-gw/pull/37)
- Return ribosome errors to caller by @jost-s in [#38](https://github.com/holochain/hc-http-gw/pull/38)
- Maximize build space right after checkout by @jost-s in [#33](https://github.com/holochain/hc-http-gw/pull/33)
- Differentiate zome call error responses by @jost-s
- Clean up fixes by @ThetaSinner in [#31](https://github.com/holochain/hc-http-gw/pull/31)
- Hook up zome call by @jost-s in [#29](https://github.com/holochain/hc-http-gw/pull/29)
- Split testing and runtime logging by @ThetaSinner in [#28](https://github.com/holochain/hc-http-gw/pull/28)
- Establish admin ws connection by @ThetaSinner in [#27](https://github.com/holochain/hc-http-gw/pull/27)
- Implement app selection logic by @ThetaSinner in [#26](https://github.com/holochain/hc-http-gw/pull/26)
- Use trait for admin and app conn by @ThetaSinner in [#23](https://github.com/holochain/hc-http-gw/pull/23)
- Simplify error responses by @jost-s in [#25](https://github.com/holochain/hc-http-gw/pull/25)
- Reject request methods other than get to valid paths by @jost-s in [#22](https://github.com/holochain/hc-http-gw/pull/22)
- Unify function_name to fn_name by @jost-s
- Move router to separate module by @jost-s
- Create DNA hashes without `fixt` by @ThetaSinner in [#21](https://github.com/holochain/hc-http-gw/pull/21)
- Add test fixture by @ThetaSinner in [#18](https://github.com/holochain/hc-http-gw/pull/18)
- Accept http get requests by @c12i in [#14](https://github.com/holochain/hc-http-gw/pull/14)
- Read and check env vars by @c12i in [#2](https://github.com/holochain/hc-http-gw/pull/2)
- Initialize project by @c12i in [#1](https://github.com/holochain/hc-http-gw/pull/1)
- Initial commit by @c12i

### Fixed

- Improve log readability by @c12i in [#24](https://github.com/holochain/hc-http-gw/pull/24)
- Unauthorized functions return status code forbidden by @jost-s
- Bring back correct error response format by @jost-s
- Fix allowed fns in integration tests by @jost-s

## First-time Contributors

* @ThetaSinner made their first contribution in [#40](https://github.com/holochain/hc-http-gw/pull/40)

* @cdunster made their first contribution in [#35](https://github.com/holochain/hc-http-gw/pull/35)

* @jost-s made their first contribution in [#38](https://github.com/holochain/hc-http-gw/pull/38)

* @c12i made their first contribution in [#24](https://github.com/holochain/hc-http-gw/pull/24)


<!-- generated by git-cliff -->
