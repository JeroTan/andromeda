// Replace executable files using a fresh inode; never rewrite a loaded Mach-O.
import {
  copyFileSync,
  chmodSync,
  mkdtempSync,
  renameSync,
  rmSync,
  constants,
} from "node:fs";
import { dirname, join } from "node:path";

export function stageExecutable(source, destination, executable = true) {
  const temporary = mkdtempSync(
    join(dirname(destination), ".andromeda-stage-"),
  );
  const file = join(temporary, "executable");
  try {
    copyFileSync(source, file, constants.COPYFILE_EXCL);
    if (executable) chmodSync(file, 0o755);
    renameSync(file, destination);
  } finally {
    rmSync(temporary, { recursive: true, force: true });
  }
}
