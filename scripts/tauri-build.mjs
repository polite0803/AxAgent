/**
 * Tauri build wrapper that disables updater artifact signing
 * when TAURI_SIGNING_PRIVATE_KEY is not set.
 *
 * When no signing key is available, this script temporarily clears
 * the updater pubkey and endpoints in tauri.conf.json so the build
 * succeeds without the signing error, while keeping the updater
 * plugin config struct valid (null/undefined causes deserialization
 * errors at runtime).
 *
 * Usage: pnpm tauri:build [--debug] [--target <target>] [--bundles <bundles>]
 */
import { execSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

const args = process.argv.slice(2);
const confPath = resolve("src-tauri/tauri.conf.json");

if (process.env.TAURI_SIGNING_PRIVATE_KEY) {
  // Private key available — run normal tauri build (updater artifacts will be signed)
  execSync(`tauri build ${args.join(" ")}`, { stdio: "inherit" });
} else {
  // No private key — temporarily clear updater pubkey to avoid signing error
  console.log("[tauri:build] TAURI_SIGNING_PRIVATE_KEY not set, clearing updater pubkey for local build");

  const original = readFileSync(confPath, "utf-8");
  const config = JSON.parse(original);

  // Keep updater struct but clear pubkey and endpoints to avoid signing requirement
  if (config.plugins?.updater) {
    config.plugins.updater.pubkey = "";
    config.plugins.updater.endpoints = [];
  }
  if (config.bundle) {
    config.bundle.createUpdaterArtifacts = false;
  }

  writeFileSync(confPath, JSON.stringify(config, null, 2) + "\n", "utf-8");

  try {
    execSync(`tauri build ${args.join(" ")}`, { stdio: "inherit" });
  } finally {
    // Restore original config
    writeFileSync(confPath, original, "utf-8");
  }
}
