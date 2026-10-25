import { stageExecutable } from "./stage-executable.mjs";
import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, existsSync, cpSync, rmSync } from "node:fs";
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
// A cross-build must not bundle an executable left by another platform.
rmSync(
  join(
    destination,
    executable === "andromeda.exe" ? "andromeda" : "andromeda.exe",
  ),
  { force: true },
);
stageExecutable(
  join(
    root,
    "target",
    target !== host ? target : "",
    debug ? "debug" : "release",
    executable,
  ),
  join(destination, executable),
  !target.includes("windows"),
);

// Keep release notices inside the native bundle as well as download archives.
const notices = join(root, "desktop", "src-tauri", "notices");
rmSync(notices, { recursive: true, force: true });
mkdirSync(notices, { recursive: true });
for (const name of ["LICENSE", "LICENSE-MIT", "LICENSE-APACHE"])
  copyFileSync(join(root, name), join(notices, name));
const collectedNotices =
  process.env.ANDROMEDA_NOTICES_DIR ||
  join(root, "release", "generated-notices");
if (existsSync(collectedNotices))
  cpSync(collectedNotices, notices, { recursive: true });
