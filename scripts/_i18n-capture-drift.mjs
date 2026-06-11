// scripts/_i18n-capture-drift.mjs — one-off helper for Task 1.4
// Captures the full i18n-check.mjs output (stdout+stderr) to a file.

import fs from "fs";
import path from "path";
import { spawnSync } from "child_process";

const OUT = process.argv[2] || "docs/baseline/i18n-drift-2026-06-11.txt";
const r = spawnSync(process.execPath, ["scripts/i18n-check.mjs"], {
  stdio: ["ignore", "pipe", "pipe"],
  encoding: "utf8",
});
fs.mkdirSync(path.dirname(OUT), { recursive: true });
fs.writeFileSync(OUT, r.stdout + r.stderr, "utf8");
console.error(`wrote ${OUT} (exit ${r.status})`);
