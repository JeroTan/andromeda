import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, chmodSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { join } from "node:path";
const root = fileURLToPath(new URL("../../", import.meta.url));
const host = execFileSync("rustc", ["-vV"], { encoding: "utf8" }).match(
  /^host: (.+)$/m,
)[1];
const target = process.env.TAURI_ENV_TARGET_TRIPLE || host;
const debug = process.env.TAURI_ENV_DEBUG === "true";
const args = ["build", "--locked", "--bin", "andromeda"];
if (!debug) args.push("--release");
if (target !== host) args.push("--target", target);
execFileSync("cargo", args, { cwd: root, stdio: "inherit" });
const executable = target.includes("windows") ? "andromeda.exe" : "andromeda";
const destination = join(root, "desktop", "src-tauri", "binaries");
mkdirSync(destination, { recursive: true });
copyFileSync(
  join(
    root,
    "target",
    target !== host ? target : "",
    debug ? "debug" : "release",
    executable,
  ),
  join(destination, executable),
);
if (!target.includes("windows"))
  chmodSync(join(destination, executable), 0o755);
