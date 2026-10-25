// Headless DMG packaging: do not open Finder or depend on .DS_Store timing.
import { execFileSync } from "node:child_process";
import {
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  rmSync,
  symlinkSync,
} from "node:fs";
import { dirname, join, resolve } from "node:path";

if (process.platform !== "darwin" || process.argv.length !== 4) {
  throw new Error(
    "Usage on macOS: node scripts/create-dmg.mjs APP.app OUTPUT.dmg",
  );
}
const app = resolve(process.argv[2]);
const output = resolve(process.argv[3]);
if (!app.endsWith(".app") || !existsSync(join(app, "Contents/Info.plist"))) {
  throw new Error("A built application bundle is required");
}
if (existsSync(output)) throw new Error("DMG output already exists");
mkdirSync(dirname(output), { recursive: true });
const temporary = mkdtempSync(join(dirname(output), ".andromeda-dmg-"));
try {
  const staged = join(temporary, "Andromeda.app");
  cpSync(app, staged, { recursive: true, verbatimSymlinks: true });
  execFileSync("codesign", ["--verify", "--deep", "--strict", staged]);
  symlinkSync("/Applications", join(temporary, "Applications"));
  execFileSync(
    "hdiutil",
    [
      "create",
      "-volname",
      "Andromeda",
      "-srcfolder",
      temporary,
      "-format",
      "UDZO",
      output,
    ],
    { stdio: "inherit" },
  );
  execFileSync("hdiutil", ["verify", output], { stdio: "inherit" });
} catch (error) {
  rmSync(output, { force: true });
  throw error;
} finally {
  rmSync(temporary, { recursive: true, force: true });
}
