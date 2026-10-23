import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import {
  chmodSync,
  mkdtempSync,
  readFileSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "node:test";

const launcher = fileURLToPath(new URL("./launcher.mjs", import.meta.url));
const skip = process.platform === "win32" ? "POSIX executable fixture" : false;

function fakeDesktop(source) {
  const directory = mkdtempSync(join(tmpdir(), "andromeda-launcher-"));
  const path = join(directory, "andromeda-desktop");
  writeFileSync(path, source);
  chmodSync(path, 0o755);
  return { directory, path };
}

function run(args, env = {}) {
  return spawnSync(process.execPath, [launcher, ...args], {
    encoding: "utf8",
    env: { ...process.env, ...env },
  });
}

test("prints the resolved desktop path", { skip }, () => {
  const fixture = fakeDesktop("#!/bin/sh\nexit 0\n");
  const result = run(["--print-path"], {
    ANDROMEDA_DESKTOP_BIN: fixture.path,
  });
  assert.equal(result.status, 0);
  assert.equal(result.stdout.trim(), fixture.path);
});

test("passes arguments through after the separator", { skip }, () => {
  const fixture = fakeDesktop('#!/bin/sh\nprintf "%s" "$*" > "$MARKER"\n');
  const marker = join(fixture.directory, "arguments.txt");
  const result = run(["--", "--device", "phone-A"], {
    ANDROMEDA_DESKTOP_BIN: fixture.path,
    MARKER: marker,
  });
  assert.equal(result.status, 0);
  assert.equal(readFileSync(marker, "utf8"), "--device phone-A");
});

test("preserves the desktop exit status", { skip }, () => {
  const fixture = fakeDesktop("#!/bin/sh\nexit 7\n");
  const result = run([], { ANDROMEDA_DESKTOP_BIN: fixture.path });
  assert.equal(result.status, 7);
});

test("rejects an unknown argument before starting the desktop", { skip }, () => {
  const fixture = fakeDesktop("#!/bin/sh\nexit 0\n");
  const result = run(["--unknown"], {
    ANDROMEDA_DESKTOP_BIN: fixture.path,
  });
  assert.equal(result.status, 2);
  assert.match(result.stderr, /unexpected arguments/);
});

test("reports a missing desktop build", { skip }, () => {
  const result = run([], {
    ANDROMEDA_DESKTOP_BIN: join(tmpdir(), "andromeda-missing-desktop"),
  });
  assert.equal(result.status, 1);
  assert.match(result.stderr, /no desktop executable found/);
});

test("passes desktop flags through without interpreting them as launcher flags", { skip }, () => {
  const fixture = fakeDesktop('#!/bin/sh\nprintf "%s" "$*" > "$MARKER"\n');
  const marker = join(fixture.directory, "desktop-flags.txt");
  const result = run(["--", "--help", "--version", "--print-path"], {
    ANDROMEDA_DESKTOP_BIN: fixture.path,
    MARKER: marker,
  });
  assert.equal(result.status, 0);
  assert.equal(readFileSync(marker, "utf8"), "--help --version --print-path");
});
