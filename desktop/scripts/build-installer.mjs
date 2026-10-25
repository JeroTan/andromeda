import { readFileSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { createRequire } from "node:module";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const args = process.argv.slice(2);
if (args.some((arg) => arg !== "--debug") || args.length > 1) {
  throw new Error("Usage: npm run installer -- [--debug]");
}
const bundle = { darwin: "dmg", win32: "nsis", linux: "deb" }[process.platform];
if (!bundle)
  throw new Error(`No installer target configured for ${process.platform}`);
const directory = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const require = createRequire(import.meta.url);
const cli = require.resolve("@tauri-apps/cli/tauri.js");
if (process.platform === "darwin") {
  execFileSync(process.execPath, [cli, "build", "--bundles", "app", ...args], {
    cwd: directory,
    stdio: "inherit",
  });
  const profile = args.includes("--debug") ? "debug" : "release";
  const root = resolve(directory, "..");
  const bundleRoot = resolve(root, "target", profile, "bundle");
  const version = JSON.parse(
    readFileSync(resolve(directory, "package.json"), "utf8"),
  ).version;
  const arch = process.arch === "arm64" ? "aarch64" : process.arch;
  const output = resolve(bundleRoot, "dmg", `Andromeda_${version}_${arch}.dmg`);
  execFileSync(
    process.execPath,
    [
      resolve(directory, "scripts/create-dmg.mjs"),
      resolve(bundleRoot, "macos/Andromeda.app"),
      output,
    ],
    { cwd: directory, stdio: "inherit" },
  );
} else {
  execFileSync(process.execPath, [cli, "build", "--bundles", bundle, ...args], {
    cwd: directory,
    stdio: "inherit",
  });
}
