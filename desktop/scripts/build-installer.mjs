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
execFileSync(process.execPath, [cli, "build", "--bundles", bundle, ...args], {
  cwd: directory,
  stdio: "inherit",
});
