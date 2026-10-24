# Reproducible release notices

Checked 2026-10-24. This is an engineering collection proposal, not a completed notices bundle or a legal-compliance determination. Inputs reviewed: [license inventory](../licenses.md), [Rust manifest](../../Cargo.toml), both npm lockfiles, and [Android dependencies](../../android-companion/app/build.gradle.kts).

## Collection contract

Produce one manifest per shipped artifact: package coordinate/version, dependency origin, artifact checksum, selected license expression, original notice paths/checksums, source location, and inclusion reason. Preserve full copyright/license/NOTICE/PATENTS texts; SPDX identifiers alone cannot reproduce them. Pin collector versions and configuration, retain raw inputs, sort output deterministically, and fail packaging for missing evidence. Reconcile against the final archive/APK, including generated JavaScript, assets, and native libraries. These are proposed release controls.

## Rust and bundled libwebp

Use a pinned `cargo-about` with a checked-in template/configuration. Run `cargo about generate --locked --fail --format json`, selecting the release manifest/features and configuring its target triple. Generate separate CLI/desktop inventories; keep an all-features workspace inventory as an audit superset. The tool supports target filtering, dependency exclusions and hashed per-crate license clarifications. Do not discard build dependencies before checking whether they embed generated code. [Generation](https://embarkstudios.github.io/cargo-about/cli/generate/index.html), [configuration](https://embarkstudios.github.io/cargo-about/cli/generate/config.html).

Explicitly supplement native packages. The locally cached, locked `libwebp-sys@0.9.6` declares MIT, but `build.rs` compiles `vendor/src` and `vendor/sharpyuv`; its vendor tree contains `COPYING`, `AUTHORS`, and `PATENTS`. Collect those exact files alongside wrapper notices and inspect nested directories/source headers. Preserve the `.crate` checksum and paths in the manifest. The publisher's [versioned crate archive](https://static.crates.io/crates/libwebp-sys/libwebp-sys-0.9.6.crate) is the reproducible source; the manifest license is insufficient evidence for bundled native code.

Avoid assuming `--offline` preserves all cargo-about evidence: its documentation warns that repository-backed clarifications may fall back to generic license text. Archive harvested originals and validate their hashes. [Offline behavior](https://embarkstudios.github.io/cargo-about/cli/generate/index.html).

## npm and bundled UI code

For each lockfile, use the pinned npm version and `npm ci` in the release build. Start inventory with `npm sbom --sbom-format=cyclonedx --omit=dev`; retain a full-lockfile inventory too. Lockfile-only mode omits additional package metadata. An SBOM is an inventory input; copy actual package license/NOTICE files separately. [npm ci](https://docs.npmjs.com/cli/commands/npm-ci/), [npm sbom](https://docs.npmjs.com/cli/commands/npm-sbom/).

Reconcile that candidate list with emitted bundle modules and generated runtime helpers; a dependency marked development-only can still contribute distributed output. Record exclusions rather than assuming every lockfile entry ships. For `@lynx-js/react@0.128.0`, the installed package's `LICENSE` is Apache-2.0 despite absent manifest metadata; keep a version/checksum-bound override backed by the [published package](https://registry.npmjs.org/@lynx-js/react/-/react-0.128.0.tgz). Review the inventory's caniuse-lite/data attribution and other special cases when their content is included.

## Android and Lynx native dependencies

Resolve `:app:dependencies --configuration releaseRuntimeClasspath`, then export resolved AAR/JAR coordinates and checksums. Add [dependency locks](https://docs.gradle.org/current/userguide/dependency_locking.html) and reviewed [verification metadata](https://docs.gradle.org/current/userguide/dependency_verification.html). A pinned [AboutLibraries plugin](https://github.com/mikepenz/AboutLibraries) can collect Gradle license metadata and accept reviewed overrides; supplement it by inspecting AARs, nested JARs, native objects, and publisher sources.

The [Lynx 4.1.1 POM](https://repo.maven.apache.org/maven2/org/lynxsdk/lynx/lynx/4.1.1/lynx-4.1.1.pom) brings seven matching modules, including PrimJS and primjsWasm. Local inspection of all eight cached AARs and their nested JARs found no filenames containing LICENSE/NOTICE/COPYING/PATENTS. They contain native libraries including `liblynx.so`, `libquick.so`, and `libwasm.so`. Thus archive scanning alone leaves a material gap.

Map these binaries to exact publisher source revisions/build inputs and recursively harvest applicable notices. Upstream examples include [Lynx's nested QuickJS license](https://github.com/lynx-family/lynx/blob/0115912417025ed8179a679d5651314160807e78/third_party/quickjs/LICENSE) and [PrimJS's V8 notice](https://github.com/lynx-family/primjs/blob/60e1b418fb2d6e18fe2231470a752b4fce07897e/LICENSE.v8). These inspected revisions demonstrate nested obligations; they are not established mappings to the 4.1.1 binaries. That mapping remains unresolved.

## MPL source provision

For MPL-covered code actually distributed, provide the corresponding source, including modifications, under MPL and tell recipients how to obtain it. Preserve notices; a license list alone does not satisfy source provision. Recommend versioned source archives with hashes beside each release and an included source-access statement. [MPL 2.0 §§3.1–3.4](https://www.mozilla.org/en-US/MPL/2.0/).
