# License and dependency review

Andromeda itself is dual-licensed under MIT or Apache-2.0; see [LICENSE-MIT](../LICENSE-MIT) and [LICENSE-APACHE](../LICENSE-APACHE). This page records the dependency review performed on 2026-10-23. It is an engineering inventory, not legal advice.

## Rust workspace

The workspace resolves 585 Rust package records with all optional features enabled, including the desktop workspace and target-specific transitive dependencies. Every declared license expression is permissive or weak-copyleft; no GPL, AGPL, SSPL or unknown license was found. Two `r-efi` records also offer LGPL-2.1-or-later as one alternative; the permissive MIT/Apache-2.0 alternative applies.

| License family | Packages | Notes |
| --- | ---: | --- |
| MIT / Apache-2.0 combinations | 360 | Includes permissive alternatives and slash forms; LLVM-exception alternatives retain their notices |
| MIT | 138 | Permissive |
| Zlib combinations | 24 | Permissive alternatives |
| Unicode-3.0 | 19 | Unicode data tables |
| Unlicense combinations | 12 | Permissive alternatives |
| BSD-2-Clause / BSD-3-Clause combinations | 16 | Permissive |
| MPL-2.0 | 5 | `cssparser`, `cssparser-macros`, `dtoa-short`, `option-ext`, `selectors` |
| Apache-2.0, including LLVM-exception form | 5 | Retain applicable exception notices |
| MIT / Apache-2.0 / LGPL-2.1-or-later | 2 | `r-efi`; select MIT or Apache-2.0, not the LGPL option |
| CC0-1.0 / MIT-0 / Apache-2.0 | 2 | Permissive alternatives |
| ISC | 1 | Permissive |
| MIT/Apache-2.0 AND NCSA | 1 | Optional `libfuzzer-sys`; both applicable notices required |

MPL-2.0 is file-level copyleft. Distributing unmodified crates is compatible with the project licenses, but a release must keep the MPL notices and make the corresponding source available. The MPL crates are transitive dependencies, not modified project source.

Reproduce the Rust inventory with:

```sh
cargo metadata --locked --all-features --format-version 1 \
  | jq -r '.packages[] | [.name,.version,(.license // "UNKNOWN")] | @tsv' \
  | sort -u
```

Image output adds `webp` (MIT OR Apache-2.0), `libwebp-sys` (MIT wrapper with bundled libwebp notices), and `image-webp` (MIT OR Apache-2.0). The optional AVIF feature adds `ravif` (BSD-3-Clause), `rav1e` (BSD-2-Clause), and their transitive dependencies. Review and include the bundled libwebp copyright/license/PATENTS files when distributing its native code; the wrapper's manifest alone is not the full notice set. These codecs do not change the release-notices gate.

## Desktop npm dependencies

`desktop/package-lock.json` resolves MIT, Apache-2.0, ISC, BSD-3-Clause and MPL-2.0 packages. The MPL-2.0 entries are `lightningcss` and its platform binaries. No GPL, AGPL, SSPL or unknown license was found.

## Android companion UI npm dependencies

`android-companion/ui/package-lock.json` contains MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, CC0-1.0, CC-BY-4.0, PSF-2.0, BlueOak-1.0.0, 0BSD and `MPL-2.0 OR Apache-2.0` packages.

Two entries need special handling in a release:

- `@lynx-js/react@0.128.0` omits a `license` field from `package.json`, but its packaged `LICENSE` file is Apache-2.0. Record it as Apache-2.0 and include the notice.
- `caniuse-lite` ships browser-support data under CC-BY-4.0; attribution is required if that data is redistributed. `argparse` uses PSF-2.0 and `sax` uses BlueOak-1.0.0; include their notices if their code is redistributed.

Reproduce the JavaScript inventory with:

```sh
for file in desktop/package-lock.json android-companion/ui/package-lock.json; do
  jq -r '.packages | to_entries[] | select(.key != "") |
    [.value.name // (.key | split("node_modules/") | last),
     .value.version // "", (.value.license // "UNKNOWN")] | @tsv' "$file" |
    sort -u
done
```

## Release obligations

A distributable release must include the project license files and a generated third-party notices file covering MIT, BSD, Apache-2.0, ISC, MPL-2.0, CC-BY-4.0, PSF-2.0 and BlueOak-1.0.0 dependencies. The current repository has the project licenses but does not yet ship a complete third-party notices bundle. That bundle is part of release packaging and artifact verification, not a code blocker for development builds.


## Collection tooling added on 2026-10-24

`scripts/collect-notices.py` and `release/about.toml` generate a macOS arm64 review candidate with pinned cargo-about 0.9.2. The first reviewed candidate contains 304 components and 193 unique notice texts. Seven missing original-file cases were resolved from exact upstream revisions; eight objc2-family crates, selectors, and final desktop bundle reconciliation remain explicit review gaps. Android/Lynx native mapping and MPL source provision also remain open. This does not claim the complete release notice obligation is met. See [packaging](release-packaging.md) and [research](research/release-notices.md).
