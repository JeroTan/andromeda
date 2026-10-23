#!/usr/bin/env node
import { createRequire } from "node:module";
import { accessSync, constants, existsSync, statSync } from "node:fs";
import { spawn } from "node:child_process";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const require = createRequire(import.meta.url);
const packageJson = require(join(root, "package.json"));
const isWindows = process.platform === "win32";
const executable = isWindows ? "andromeda-desktop.exe" : "andromeda-desktop";
const app = process.platform === "darwin" ? "Andromeda.app" : null;

function help() {
  console.log(`Andromeda ${packageJson.version}

Usage:
  andromeda [--print-path] [-- <desktop arguments>]

Opens the Andromeda desktop interface. The launcher resolves, in order:
  1. ANDROMEDA_DESKTOP_BIN
  2. ANDROMEDA_DESKTOP_APP (macOS application bundle)
  3. @andromeda/desktop-<platform>-<arch> beside this package
  4. a local desktop build under desktop/src-tauri/target

Development:
  cargo build --locked --bin andromeda
  npm run tauri build --prefix desktop -- --debug

Environment:
  ANDROMEDA_DESKTOP_BIN  absolute path to the desktop executable
  ANDROMEDA_DESKTOP_APP  absolute path to Andromeda.app on macOS
`);
}

function candidates() {
  const paths = [];
  try {
    const platformPackage = `@andromeda/desktop-${process.platform}-${process.arch}`;
    const packageRoot = dirname(
      require.resolve(`${platformPackage}/package.json`, { paths: [root] }),
    );
    if (app)
      paths.push(join(packageRoot, app, "Contents", "MacOS", executable));
    paths.push(join(packageRoot, "bin", executable));
    paths.push(join(packageRoot, executable));
  } catch {
    // The platform package is optional until a release publishes it.
  }
  if (app) {
    for (const target of [
      join(root, "target"),
      join(root, "desktop", "src-tauri", "target"),
    ]) {
      paths.push(
        join(
          target,
          "release",
          "bundle",
          "macos",
          app,
          "Contents",
          "MacOS",
          executable,
        ),
        join(
          target,
          "debug",
          "bundle",
          "macos",
          app,
          "Contents",
          "MacOS",
          executable,
        ),
      );
    }
  }
  paths.push(
    join(root, "desktop", "src-tauri", "target", "release", executable),
    join(root, "desktop", "src-tauri", "target", "debug", executable),
    join(root, "target", "release", executable),
    join(root, "target", "debug", executable),
  );
  return [...new Set(paths)];
}

function resolveCandidate(candidate) {
  if (!candidate || !existsSync(candidate)) return null;
  try {
    const path =
      app && statSync(candidate).isDirectory() && candidate.endsWith(app)
        ? join(candidate, "Contents", "MacOS", executable)
        : candidate;
    if (!existsSync(path) || !statSync(path).isFile()) return null;
    if (!isWindows) accessSync(path, constants.X_OK);
    return path;
  } catch {
    return null;
  }
}

function resolveDesktop() {
  const explicit =
    process.env.ANDROMEDA_DESKTOP_BIN || process.env.ANDROMEDA_DESKTOP_APP;
  if (explicit) return resolveCandidate(explicit);
  for (const candidate of candidates()) {
    const path = resolveCandidate(candidate);
    if (path) return path;
  }
  return null;
}

const args = process.argv.slice(2);
if (args.includes("--help") || args.includes("-h")) {
  help();
  process.exit(0);
}
if (args.includes("--version") || args.includes("-v")) {
  console.log(packageJson.version);
  process.exit(0);
}
const desktop = resolveDesktop();
if (!desktop) {
  console.error(`andromeda: no desktop executable found.

Build the interface first:
  cargo build --locked --bin andromeda
  npm run tauri build --prefix desktop -- --debug

Or set ANDROMEDA_DESKTOP_BIN to an absolute executable path.`);
  process.exit(1);
}
if (args.includes("--print-path")) {
  console.log(desktop);
  process.exit(0);
}
const separator = args.indexOf("--");
const passthrough = separator === -1 ? [] : args.slice(separator + 1);
const unexpected = separator === -1 ? args : args.slice(0, separator);
if (unexpected.length > 0) {
  console.error(`andromeda: unexpected arguments: ${unexpected.join(" ")}\n`);
  help();
  process.exit(2);
}

let child;
try {
  child = spawn(desktop, passthrough, { stdio: "inherit" });
} catch (error) {
  console.error(`andromeda: could not start ${desktop}: ${error.message}`);
  process.exit(1);
}
child.on("error", (error) => {
  console.error(`andromeda: could not start ${desktop}: ${error.message}`);
  process.exit(1);
});
child.on("exit", (code, signal) => {
  if (signal) process.kill(process.pid, signal);
  process.exit(code ?? 1);
});
