import test from "node:test";
import assert from "node:assert/strict";
import {
  mkdtempSync,
  writeFileSync,
  openSync,
  readFileSync,
  closeSync,
  rmSync,
  readdirSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { stageExecutable } from "./stage-executable.mjs";

test("updating an executable preserves bytes held by the old running inode", () => {
  const directory = mkdtempSync(join(tmpdir(), "andromeda-inode-"));
  const installed = join(directory, "andromeda");
  const source = join(directory, "candidate");
  writeFileSync(installed, "original executable bytes");
  const old = openSync(installed, "r");
  try {
    writeFileSync(source, "updated executable bytes");
    stageExecutable(source, installed);
    assert.equal(readFileSync(old, "utf8"), "original executable bytes");
    assert.equal(readFileSync(installed, "utf8"), "updated executable bytes");
    assert.deepEqual(readdirSync(directory).sort(), ["andromeda", "candidate"]);
  } finally {
    closeSync(old);
    rmSync(directory, { recursive: true, force: true });
  }
});
