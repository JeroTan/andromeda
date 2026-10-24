# Android native notice sources

Checked 2026-10-24 for the existing Lynx 4.1.1 companion APK. This note records usable publisher evidence and remaining provenance uncertainty; it does not establish additional release requirements or a legal-compliance conclusion.

## Exact published artifact evidence

The [Lynx 4.1.1 POM](https://repo.maven.apache.org/maven2/org/lynxsdk/lynx/lynx/4.1.1/lynx-4.1.1.pom) selects `lynx-base`, `lynx-gfx`, `lynx-trace`, `primjs`, `primjsWasm`, `lynx-jssdk`, and `service-api`, all at 4.1.1. Its license declaration is Apache-2.0. The [PrimJS POM](https://repo.maven.apache.org/maven2/org/lynxsdk/lynx/primjs/4.1.1/primjs-4.1.1.pom) likewise declares Apache-2.0. Neither POM identifies a source commit.

The exact [Lynx source JAR](https://repo.maven.apache.org/maven2/org/lynxsdk/lynx/lynx/4.1.1/lynx-4.1.1-sources.jar) supplies original notices that were absent from the inspected binary AAR and nested `classes.jar`. Its SHA-256 is `1ac0da6c820a06428cbf76337719275d5368d6898525d2880d2173a6343fe818`. Extract these entries verbatim into the Android notices bundle:

| Source-JAR entry | SHA-256 |
| --- | --- |
| `com/lynx/tasm/behavior/LICENSE` | `43070e2d4e532684de521b885f385d0841030efa2b1a20bafb76133a5e1379c1` |
| `com/lynx/tasm/behavior/NOTICE` | `5cfae5780d39ffd8b6de6bddacb6385ae03f628f7ac47b49d6a835e1c551733a` |
| `com/lynx/tasm/utils/LICENSE` | `da6d3703ed11cbe42bd212c725957c98da23cbff1998c05fa4b3d976d1a58e93` |

These contain Apache-2.0, the publisher's Android-derived attribution file, and Meta's MIT notice respectively. Preserve the attribution file as supplied rather than inferring which historical entries to remove. The empty `MODULE_LICENSE_APACHE2` marker adds no text. This closes a concrete collection gap using the exact published version, without requiring a repository revision match.

## PrimJS release mapping and originals

The publisher's [4.1.1 tag reference](https://api.github.com/repos/lynx-family/primjs/git/ref/tags/4.1.1) resolves to `8d129eab500c86e3b2171144e45d3f19e5dce421`. Its [release workflow](https://github.com/lynx-family/primjs/blob/8d129eab500c86e3b2171144e45d3f19e5dce421/.github/workflows/android-release.yml) derives the publishing version from the tag and passes it to Gradle. This is a publisher release association, not a reproduced binary build.

Use the originals at that immutable revision: [Apache LICENSE](https://github.com/lynx-family/primjs/blob/8d129eab500c86e3b2171144e45d3f19e5dce421/LICENSE), [PrimJS NOTICE](https://github.com/lynx-family/primjs/blob/8d129eab500c86e3b2171144e45d3f19e5dce421/NOTICE), [Node.js MIT notice](https://github.com/lynx-family/primjs/blob/8d129eab500c86e3b2171144e45d3f19e5dce421/LICENSE.nodejs), and [V8 BSD notice](https://github.com/lynx-family/primjs/blob/8d129eab500c86e3b2171144e45d3f19e5dce421/LICENSE.v8).

QuickJS is vendored in the same revision, not fetched as a separate dependency: its [source header](https://github.com/lynx-family/primjs/blob/8d129eab500c86e3b2171144e45d3f19e5dce421/src/interpreter/quickjs/source/quickjs.cc) preserves Bellard/Gordon's complete MIT grant followed by Lynx's Apache notice. Preserve that header with the source URL. Wasm3 is also vendored; its [MIT LICENSE](https://github.com/lynx-family/primjs/blob/8d129eab500c86e3b2171144e45d3f19e5dce421/third_party/wasm3/LICENSE) names Steven Massey and Volodymyr Shymanskyy. The [Android Wasm module](https://github.com/lynx-family/primjs/blob/8d129eab500c86e3b2171144e45d3f19e5dce421/Android/PrimjsWasm/build.gradle) builds the `wasm` target. These are concrete nested notices to retain.

The same revision's [DEPS](https://github.com/lynx-family/primjs/blob/8d129eab500c86e3b2171144e45d3f19e5dce421/DEPS) selects V8 headers 11.1.277.1 and Android V8 AAR 11.1.277.2. However, [Android packaging](https://github.com/lynx-family/primjs/blob/8d129eab500c86e3b2171144e45d3f19e5dce421/Android/app/build.gradle) excludes `libv8_libfull.cr.so`; do not equate a build dependency with a redistributed V8 engine. The inspected PrimJS AAR contains adapter libraries and `libquick.so`.

## Boundaries of the evidence

The checked [Lynx tag listing](https://api.github.com/repos/lynx-family/lynx/tags?per_page=100) contains 4.1.0 and 4.1.2 but no 4.1.1. The 4.1.2 source's [dependency file](https://github.com/lynx-family/lynx/blob/0115912417025ed8179a679d5651314160807e78/dependencies/DEPS) pins PrimJS `eb435db4ac9d07bb97bd08a754dd9bde25d41695`; it must not be relabeled as the 4.1.1 build mapping. Native-string inspection found compiler revision identifiers, not a usable Lynx source commit. Retain the exact source JAR association and record the unresolved root revision rather than inventing one.

Absent embedded full text does not itself establish a prohibition on distribution. License declarations, incorporated references, original notices, and provenance are separate evidence. The practical next step is to bundle the verified originals above alongside the existing inventory, with these mapping qualifications preserved.
