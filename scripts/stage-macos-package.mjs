#!/usr/bin/env node
// Stage private local preview packages. Publication/signing are separate release steps.
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repo = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const input = process.argv.slice(2);
const options = new Map();
for (let i = 0; i < input.length; i += 2) {
  if (
    !["--app", "--out", "--notices", "--scope"].includes(input[i]) ||
    !input[i + 1] ||
    options.has(input[i])
  ) {
    throw new Error(
      "Usage: node scripts/stage-macos-package.mjs --app /path/Andromeda.app --out NEW_DIRECTORY --notices NOTICE_DIRECTORY [--scope @owner]",
    );
  }
  options.set(input[i], input[i + 1]);
}
for (const required of ["--app", "--out", "--notices"])
  if (!options.has(required)) throw new Error(`Missing ${required}`);
if (process.platform !== "darwin")
  throw new Error("Run this macOS staging command on macOS");
const app = resolve(options.get("--app"));
const out = resolve(options.get("--out"));
const notices = resolve(options.get("--notices"));
const scope = options.get("--scope") ?? "@jrw";
if (!/^@[a-z0-9][a-z0-9-]*$/.test(scope))
  throw new Error("Scope must be a lowercase npm scope such as @owner");
if (existsSync(out))
  throw new Error("Output already exists; refusing to overwrite");
if (basename(app) !== "Andromeda.app" || !statSync(app).isDirectory())
  throw new Error("Expected an Andromeda.app bundle");
if (
  !statSync(notices).isDirectory() ||
  !existsSync(join(notices, "inventory.json"))
)
  throw new Error("Notices directory must contain inventory.json");
const noticeInventory = JSON.parse(
  readFileSync(join(notices, "inventory.json"), "utf8"),
);
const manifest = JSON.parse(readFileSync(join(repo, "package.json"), "utf8"));
const executable = join(app, "Contents/MacOS/andromeda-desktop");
const cli = join(app, "Contents/Resources/bin/andromeda");
function archOf(path) {
  const bytes = readFileSync(path);
  if (bytes.length < 8 || bytes.readUInt32LE(0) !== 0xfeedfacf)
    throw new Error(`Expected thin 64-bit Mach-O: ${path}`);
  const cpu = bytes.readUInt32LE(4);
  if (cpu === 0x0100000c) return "arm64";
  if (cpu === 0x01000007) return "x64";
  throw new Error(`Unsupported Mach-O architecture: ${path}`);
}
const arch = archOf(executable);
if (archOf(cli) !== arch)
  throw new Error("Desktop and bundled MCP architectures differ");
const version = manifest.version;
if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(version))
  throw new Error("Invalid package version");
const bundleVersion = execFileSync(
  "/usr/libexec/PlistBuddy",
  ["-c", "Print :CFBundleShortVersionString", join(app, "Contents/Info.plist")],
  { encoding: "utf8" },
).trim();
if (bundleVersion !== version)
  throw new Error("App and launcher versions differ");
if (
  execFileSync(cli, ["--version"], { encoding: "utf8" }).trim() !==
  `andromeda-mcp ${version}`
)
  throw new Error("Bundled CLI version differs");
const packageName = `${scope}/andromeda`;
const platformName = `${scope}/desktop-darwin-${arch}`;
const work = mkdtempSync(join(tmpdir(), "andromeda-package-"));
const output = join(work, "output");
mkdirSync(output);
const hash = (path) =>
  createHash("sha256").update(readFileSync(path)).digest("hex");
function files(path, prefix = "") {
  return readdirSync(path, { withFileTypes: true })
    .flatMap((entry) => {
      const relative = prefix ? `${prefix}/${entry.name}` : entry.name;
      if (entry.isSymbolicLink())
        throw new Error(`npm does not preserve bundle symlinks: ${relative}`);
      if (entry.isDirectory()) return files(join(path, entry.name), relative);
      if (!entry.isFile())
        throw new Error(`Unsupported file type: ${relative}`);
      return [relative];
    })
    .sort();
}
function pack(directory) {
  const result = JSON.parse(
    execFileSync(
      "npm",
      ["pack", "--ignore-scripts", "--json", "--pack-destination", output],
      { cwd: directory, encoding: "utf8" },
    ),
  )[0];
  const archive = join(output, result.filename);
  const extracted = mkdtempSync(join(work, "unpacked-"));
  execFileSync("tar", ["-xzf", archive, "-C", extracted]);
  return {
    filename: result.filename,
    sha256: hash(archive),
    integrity: result.integrity,
    extracted: join(extracted, "package"),
  };
}
try {
  const platformRoot = join(work, "platform");
  mkdirSync(platformRoot);
  cpSync(app, join(platformRoot, "Andromeda.app"), { recursive: true });
  cpSync(notices, join(platformRoot, "third-party-notices"), {
    recursive: true,
  });
  for (const name of ["LICENSE", "LICENSE-MIT", "LICENSE-APACHE"])
    cpSync(join(repo, name), join(platformRoot, name));
  writeFileSync(
    join(platformRoot, "package.json"),
    JSON.stringify(
      {
        name: platformName,
        version,
        private: true,
        description:
          "Andromeda local macOS preview; not a signed public release",
        os: ["darwin"],
        cpu: [arch],
        license: "MIT OR Apache-2.0",
        files: ["Andromeda.app/**", "third-party-notices/**", "LICENSE*"],
      },
      null,
      2,
    ),
  );
  const appFiles = files(app);
  const platformPack = pack(platformRoot);
  const extractedApp = join(platformPack.extracted, "Andromeda.app");
  if (JSON.stringify(files(extractedApp)) !== JSON.stringify(appFiles))
    throw new Error("npm archive changed the app file list");
  for (const file of appFiles) {
    if (hash(join(app, file)) !== hash(join(extractedApp, file)))
      throw new Error(`npm archive changed app bytes: ${file}`);
    if (
      (statSync(join(app, file)).mode & 0o111) !==
      (statSync(join(extractedApp, file)).mode & 0o111)
    )
      throw new Error(`npm archive changed executable bits: ${file}`);
  }
  const launcherRoot = join(work, "launcher");
  mkdirSync(join(launcherRoot, "npm"), { recursive: true });
  cpSync(
    join(repo, "npm/launcher.mjs"),
    join(launcherRoot, "npm/launcher.mjs"),
  );
  cpSync(join(repo, "npm/README.md"), join(launcherRoot, "README.md"));
  for (const name of ["LICENSE", "LICENSE-MIT", "LICENSE-APACHE"])
    cpSync(join(repo, name), join(launcherRoot, name));
  const launcherManifest = {
    name: packageName,
    version,
    private: true,
    description: "Andromeda local desktop launcher preview",
    type: "module",
    bin: { andromeda: "npm/launcher.mjs" },
    engines: { node: ">=20" },
    license: "MIT OR Apache-2.0",
    andromedaPlatformScope: scope,
    optionalDependencies: { [platformName]: version },
    files: ["npm/launcher.mjs", "README.md", "LICENSE*"],
  };
  writeFileSync(
    join(launcherRoot, "package.json"),
    JSON.stringify(launcherManifest, null, 2),
  );
  const launcherPack = pack(launcherRoot);
  if (
    hash(join(repo, "npm/launcher.mjs")) !==
    hash(join(launcherPack.extracted, "npm/launcher.mjs"))
  )
    throw new Error("Launcher changed in npm archive");
  const zipName = `Andromeda-${version}-darwin-${arch}-preview.zip`;
  // The downloadable archive includes project and dependency notices beside the unchanged app.
  const zipRoot = join(work, "Andromeda");
  mkdirSync(zipRoot);
  for (const name of [
    "Andromeda.app",
    "third-party-notices",
    "LICENSE",
    "LICENSE-MIT",
    "LICENSE-APACHE",
  ])
    cpSync(join(platformRoot, name), join(zipRoot, name), { recursive: true });
  execFileSync("ditto", [
    "-c",
    "-k",
    "--keepParent",
    zipRoot,
    join(output, zipName),
  ]);
  const artifacts = [platformPack, launcherPack].map(
    ({ extracted, ...artifact }) => artifact,
  );
  artifacts.push({ filename: zipName, sha256: hash(join(output, zipName)) });
  writeFileSync(
    join(output, "SHA256SUMS"),
    artifacts.map((a) => `${a.sha256}  ${a.filename}`).join("\n") + "\n",
  );
  writeFileSync(
    join(output, "manifest.json"),
    JSON.stringify(
      {
        schema_version: 1,
        version,
        platform: "darwin",
        arch,
        launcher_package: packageName,
        platform_package: platformName,
        private_preview: true,
        ready_to_publish: false,
        notices_complete: noticeInventory.complete === true,
        staging_commit: execFileSync("git", ["rev-parse", "HEAD"], {
          cwd: repo,
          encoding: "utf8",
        }).trim(),
        staging_tree_dirty: Boolean(
          execFileSync("git", ["status", "--porcelain"], {
            cwd: repo,
            encoding: "utf8",
          }).trim(),
        ),
        app_source_revision: null,
        app_files: appFiles.length,
        desktop_sha256: hash(executable),
        cli_sha256: hash(cli),
        artifacts,
        limitations: [
          "Private local package staging does not establish npm scope ownership.",
          "Staging Git metadata identifies this packaging checkout; app source revision is not inferred from a supplied binary.",
          "Signing/notarization and full release acceptance must be checked separately.",
          "Notices completeness is reported from the supplied inventory; staging does not resolve missing licenses.",
        ],
      },
      null,
      2,
    ) + "\n",
  );
  mkdirSync(dirname(out), { recursive: true });
  cpSync(output, out, { recursive: true, errorOnExist: true, force: false });
  console.log(
    JSON.stringify({
      output: out,
      artifacts: artifacts.map((a) => a.filename),
      app_files_verified: appFiles.length,
      private_preview: true,
      notices_complete: noticeInventory.complete === true,
    }),
  );
} finally {
  rmSync(work, { recursive: true, force: true });
}
