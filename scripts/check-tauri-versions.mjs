#!/usr/bin/env node
// Fails when a Tauri crate and its npm package are on different major.minor
// releases. `tauri build` refuses to build in that state ("Found version
// mismatched Tauri packages"), but nothing before the release build runs it, so
// a lockfile update to one side only passed the tests and broke the release
// (v0.1.2). This makes the same comparison in the test job instead.
//
// Compares the versions actually installed: node_modules (run after
// `pnpm install`) against src-tauri/Cargo.lock.
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");

const pkg = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
const npmNames = Object.keys({ ...pkg.dependencies, ...pkg.devDependencies })
  // @tauri-apps/cli is the build tool, not the runtime counterpart of a crate
  .filter((name) => name.startsWith("@tauri-apps/") && name !== "@tauri-apps/cli");

// Cargo.lock lists each package as `name = "..."` followed by `version = "..."`.
// A crate can appear twice when two versions are in the tree; keep them all.
const crates = new Map();
const lock = readFileSync(join(root, "src-tauri", "Cargo.lock"), "utf8");
for (const [, name, version] of lock.matchAll(/^name = "([^"]+)"\nversion = "([^"]+)"$/gm)) {
  crates.set(name, [...(crates.get(name) ?? []), version]);
}

const majorMinor = (version) => version.split(".").slice(0, 2).join(".");

const mismatches = [];
for (const npmName of npmNames) {
  // @tauri-apps/api <-> tauri, @tauri-apps/plugin-foo <-> tauri-plugin-foo
  const short = npmName.slice("@tauri-apps/".length);
  const crateName = short === "api" ? "tauri" : `tauri-${short}`;
  const crateVersions = crates.get(crateName);
  if (!crateVersions) {
    continue; // a JS-only package with no crate to match
  }
  let npmVersion;
  try {
    npmVersion = JSON.parse(
      readFileSync(join(root, "node_modules", npmName, "package.json"), "utf8"),
    ).version;
  } catch {
    console.error(`::error::${npmName} is not installed; run pnpm install first`);
    process.exit(1);
  }
  for (const crateVersion of crateVersions) {
    const ok = majorMinor(crateVersion) === majorMinor(npmVersion);
    console.log(`${ok ? "ok  " : "NG  "} ${crateName} (v${crateVersion}) : ${npmName} (v${npmVersion})`);
    if (!ok) {
      mismatches.push(crateName);
    }
  }
}

if (mismatches.length > 0) {
  console.error(
    "::error::Tauri crates and npm packages are on different major.minor releases; " +
      "`tauri build` will refuse to build. Update both sides together.",
  );
  process.exit(1);
}
