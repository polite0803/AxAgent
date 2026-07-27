#!/usr/bin/env node
// check-upgrade-compat.mjs
// ---------------------------------------------------------------------------
// Watches known "upstream-compat" workarounds in AxAgent and detects when the
// official fix lands, so we can upgrade / fold the workaround back promptly.
//
// Two categories:
//
//  A. TypeScript 7 npm peer conflicts (typedoc, typescript-eslint, ...)
//     Cause: project bumped `typescript` to ^7.0.2 (2026-07-15, 8f5fd252).
//     Some dev tooling still pins its `typescript` peer to 5.x/6.x and breaks
//     a plain `npm install` with ERESOLVE. We wait for a STABLE npm release
//     whose `typescript` peer allows ^7, then upgrade + drop --legacy-peer-deps.
//
//  B. Rust clippy ICE (rustc 1.97.0) two-stage split
//     Cause: `cargo clippy -D warnings` hits an Internal Compiler Error on
//     `axagent-disk-cache` / `axagent-rt-theme` under rustc 1.97.0.
//     CI works around it by excluding those crates from the normal stage and
//     running them separately with `--allow clippy::all` (.github/workflows/ci.yml).
//     We watch for a newer rustc stable that may fix the ICE, then fold them back.
//
// Exit codes:
//   0  -> nothing actionable yet (still waiting)
//   42 -> an ACTION is available (stable upgrade / fold-back possible) — alert!
//   1  -> script error (network / parse failure)
//
// Look for "STATUS: ACTION NEEDED" in output to decide whether to notify.
// ---------------------------------------------------------------------------

import { execSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";

const ROOT = process.cwd();
const TS_MAJOR = 7;

// --- minimal semver range evaluator (covers npm ranges we encounter) ---------
function parseVer(v) {
  const m = /^(\d+)\.(\d+)\.(\d+)/.exec(v || "");
  return m ? [+m[1], +m[2], +m[3]] : [0, 0, 0];
}
function cmp(a, b) {
  for (let i = 0; i < 3; i++) {
    if (a[i] !== b[i]) { return a[i] < b[i] ? -1 : 1; }
  }
  return 0;
}
function expandComparator(comp) {
  comp = comp.trim();
  if (comp === "*" || comp === "x" || comp === "") { return { min: [0, 0, 0], max: [999, 999, 999] }; }
  let op = "";
  let ver = comp;
  const m = /^([<>=^~]*)\s*(.+)$/.exec(comp);
  if (m) {
    op = m[1];
    ver = m[2];
  }
  if (/x|\*/.test(ver)) {
    const parts = ver.split(".");
    if (parts[0] === "x" || parts[0] === "*") { return { min: [0, 0, 0], max: [999, 999, 999] }; }
    const maj = +parts[0];
    if (parts[1] === "x" || parts[1] === "*" || parts[1] === undefined) {
      return { min: [maj, 0, 0], max: [maj + 1, 0, 0] };
    }
    const min2 = +parts[1];
    if (parts[2] === "x" || parts[2] === "*" || parts[2] === undefined) {
      return { min: [maj, min2, 0], max: [maj, min2 + 1, 0] };
    }
  }
  const pv = parseVer(ver);
  if (op === "^") {
    if (pv[0] > 0) { return { min: [pv[0], 0, 0], max: [pv[0] + 1, 0, 0] }; }
    if (pv[1] > 0) { return { min: [0, pv[1], 0], max: [0, pv[1] + 1, 0] }; }
    return { min: [0, 0, pv[2]], max: [0, 0, pv[2] + 1] };
  }
  if (op === "~") { return { min: [pv[0], pv[1], 0], max: [pv[0], pv[1] + 1, 0] }; }
  if (op === ">=") { return { min: pv, max: [999, 999, 999] }; }
  if (op === ">") { return { min: pv, max: [999, 999, 999], strictMin: true }; }
  if (op === "<=") { return { min: [0, 0, 0], max: pv }; }
  if (op === "<") { return { min: [0, 0, 0], max: pv, strictMax: true }; }
  return { min: pv, max: [pv[0], pv[1], pv[2] + 1] };
}
function rangeAllowsMajor(range, major) {
  if (!range || range === "*" || range === "x") { return true; }
  const parts = range.split("||").map((s) => s.trim()).filter(Boolean);
  for (const part of parts) {
    const comps = part.split(/\s+/).filter(Boolean);
    let lo = [0, 0, 0];
    let hi = [999, 999, 999];
    let strictLo = false;
    let strictHi = false;
    for (const c of comps) {
      const e = expandComparator(c);
      if (cmp(e.min, lo) > 0) { lo = e.min; }
      if (e.strictMin) { strictLo = true; }
      if (cmp(e.max, hi) < 0) { hi = e.max; }
      if (e.strictMax) { strictHi = true; }
    }
    const candidates = [[major, 0, 0]];
    if (major + 1 < 999) { candidates.push([major, 9, 0]); }
    for (const tv of candidates) {
      if (cmp(tv, lo) < 0) { continue; }
      if (strictLo && cmp(tv, lo) === 0) { continue; }
      if (cmp(tv, hi) >= 0) { continue; }
      if (strictHi && cmp(tv, hi) === 0) { continue; }
      return true;
    }
  }
  return false;
}
const isPrerelease = (v) => v.includes("-");
function cmpVer(a, b) {
  const c = cmp(parseVer(a), parseVer(b));
  if (c !== 0) { return c; }
  const ra = isPrerelease(a);
  const rb = isPrerelease(b);
  if (ra !== rb) { return ra ? -1 : 1; }
  return 0;
}

// --- npm + local helpers -----------------------------------------------------
function npmViewJson(spec, field) {
  const cmd = field ? `npm view ${spec} ${field} --json` : `npm view ${spec} --json`;
  try {
    const out = execSync(cmd, { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"], timeout: 60000 }).trim();
    return out ? JSON.parse(out) : null;
  } catch {
    return null;
  }
}
function npmViewField(spec, field) {
  try {
    const out = execSync(`npm view ${spec} ${field} --json`, {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
      timeout: 60000,
    }).trim();
    if (!out) { return null; }
    try {
      const j = JSON.parse(out);
      // For a dotted scalar field npm returns a JSON-encoded primitive;
      // return the parsed value (string/number) so callers get clean data.
      if (j && typeof j === "object" && field in j) { return j[field]; }
      return j;
    } catch {
      return out;
    }
  } catch {
    return null;
  }
}
function getInstalledVersion(pkg) {
  const p = `${ROOT}/node_modules/${pkg}/package.json`;
  if (!existsSync(p)) { return null; }
  try {
    return JSON.parse(readFileSync(p, "utf8")).version;
  } catch {
    return null;
  }
}
async function fetchLatestRustc() {
  // Primary: latest stable from rust-lang dist channel.
  try {
    const res = await fetch("https://static.rust-lang.org/dist/channel-rust-stable.toml");
    if (res.ok) {
      const text = await res.text();
      const m = text.match(/\[pkg\.rust\]\s*version\s*=\s*"([\d.]+)"/);
      if (m) { return m[1]; }
    }
  } catch {
    /* fall through to local rustc */
  }
  // Fallback: locally installed rustc (proxy when offline / no network).
  try {
    const out = execSync("rustc --version", { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"], timeout: 15000 })
      .trim();
    const m = out.match(/rustc\s+(\d+\.\d+\.\d+)/);
    return m ? m[1] : null;
  } catch {
    return null;
  }
}

// --- A. TS7 npm peer conflicts ------------------------------------------------
const TS7_WATCH = [
  { pkg: "typedoc", reason: "peer typescript 5.x||6.0.x — blocks `npm run docs` and plain `npm install` (ERESOLVE)" },
  { pkg: "typescript-eslint", reason: "peer typescript >=4.8.4 <6.1.0 — blocks lint/typecheck under TS7" },
  { pkg: "@typescript-eslint/eslint-plugin", reason: "peer typescript >=4.8.4 <6.1.0 — blocks lint under TS7" },
  { pkg: "@typescript-eslint/parser", reason: "peer typescript >=4.8.4 <6.1.0 — blocks lint under TS7" },
];

function checkTs7(lines) {
  let action = false;
  for (const item of TS7_WATCH) {
    const meta = npmViewJson(item.pkg, "");
    if (!meta || !Array.isArray(meta.versions)) {
      lines.push(`  [SKIP] ${item.pkg}: could not fetch npm metadata (offline?)`);
      continue;
    }
    const all = meta.versions.slice().sort((a, b) => -cmpVer(a, b));
    const stable = all.filter((v) => !isPrerelease(v));
    const latest = meta["dist-tags"]?.latest;
    const latestPeer = meta.peerDependencies?.typescript;
    const installed = getInstalledVersion(item.pkg);
    const installedPeer = installed ? npmViewField(`${item.pkg}@${installed}`, "peerDependencies.typescript") : null;

    const highestStable = stable[0];
    const highestStablePeer = highestStable
      ? npmViewField(`${item.pkg}@${highestStable}`, "peerDependencies.typescript")
      : null;
    const highestOverall = all[0];
    const highestOverallPeer = highestOverall
      ? npmViewField(`${item.pkg}@${highestOverall}`, "peerDependencies.typescript")
      : null;

    if (highestStablePeer && rangeAllowsMajor(highestStablePeer, TS_MAJOR)) {
      action = true;
      lines.push(`  [ACTION] ${item.pkg}`);
      lines.push(`          installed : ${installed || "n/a"} (peer ts: ${installedPeer || "n/a"})`);
      lines.push(`          latest    : ${latest} (peer ts: ${latestPeer || "n/a"})`);
      lines.push(`          UPGRADE  -> ${highestStable} (STABLE, supports TS7)`);
    } else if (
      highestOverallPeer && rangeAllowsMajor(highestOverallPeer, TS_MAJOR) && highestOverall !== highestStable
    ) {
      lines.push(`  [PROGRESS] ${item.pkg}`);
      lines.push(`          installed : ${installed || "n/a"} (peer ts: ${installedPeer || "n/a"})`);
      lines.push(`          latest    : ${latest} (peer ts: ${latestPeer || "n/a"})`);
      lines.push(`          ${highestOverall} (prerelease) supports TS7 — not stable yet, keep waiting`);
    } else {
      lines.push(`  [WAIT] ${item.pkg}`);
      lines.push(`          installed : ${installed || "n/a"} (peer ts: ${installedPeer || "n/a"})`);
      lines.push(`          latest    : ${latest} (peer ts: ${latestPeer || "n/a"}) — no TS7 support`);
    }
    lines.push(`          why: ${item.reason}`);
    lines.push("");
  }
  return action;
}

// --- B. Rust clippy ICE two-stage split --------------------------------------
const CLIPPY = {
  ciFile: ".github/workflows/ci.yml",
  iceCrates: ["axagent-disk-cache", "axagent-rt-theme"],
  bug: "rustc 1.97.0 clippy Internal Compiler Error (ICE)",
};

function parseCiClippy(ciPath) {
  const txt = readFileSync(ciPath, "utf8");
  const tm = txt.match(/toolchain:\s*"([\d.]+)"/);
  const toolchain = tm ? tm[1] : null;
  const excludes = [...txt.matchAll(/--exclude\s+([\w-]+)/g)].map((m) => m[1]);
  return { toolchain, excludes };
}

async function checkClippy(lines) {
  const ciPath = `${ROOT}/${CLIPPY.ciFile}`;
  if (!existsSync(ciPath)) {
    lines.push(`  [SKIP] clippy: ${CLIPPY.ciFile} not found`);
    return false;
  }
  const { toolchain, excludes } = parseCiClippy(ciPath);
  const stillSplit = CLIPPY.iceCrates.every((c) => excludes.includes(c));
  const latest = await fetchLatestRustc();

  lines.push(`  rustc pinned in CI : ${toolchain || "n/a"}`);
  lines.push(`  ICE crates        : ${CLIPPY.iceCrates.join(", ")}`);
  lines.push(`  latest rustc stable: ${latest || "n/a (offline?)"}`);

  if (!stillSplit) {
    lines.push(`  [RESOLVED] clippy: the two ICE crates are no longer excluded — workaround removed.`);
    lines.push("");
    return false;
  }
  if (latest && toolchain && cmpVer(latest, toolchain) > 0) {
    lines.push(`  [ACTION] clippy: newer rustc ${latest} > pinned ${toolchain} available.`);
    lines.push(`           Verify the ICE is fixed, then fold ${CLIPPY.iceCrates.join(", ")} back into the`);
    lines.push(`           normal \`cargo clippy ... -D warnings\` stage and drop the \`--allow clippy::all\` split.`);
    lines.push("");
    return true;
  }
  lines.push(`  [WAIT] clippy: still on rustc ${toolchain}, no newer stable — keep the two-stage split.`);
  lines.push(`           bug: ${CLIPPY.bug}`);
  lines.push("");
  return false;
}

// --- main --------------------------------------------------------------------
async function main() {
  console.log("=== AxAgent upgrade-compat watch ===");
  console.log("");
  console.log("--- A. TypeScript 7 npm peer conflicts ---");
  const lines = [];
  const ts7Action = checkTs7(lines);
  console.log(lines.join("\n"));

  console.log("--- B. Rust clippy ICE two-stage split ---");
  const clipLines = [];
  const clipAction = await checkClippy(clipLines);
  console.log(clipLines.join("\n"));

  const actionNeeded = ts7Action || clipAction;
  if (actionNeeded) {
    console.log("STATUS: ACTION NEEDED — an upstream-compatible release is available.");
    console.log("Next steps: apply the upgrade / fold the workaround back, then re-run CI.");
    process.exit(42);
  } else {
    console.log("STATUS: WAITING — no actionable upstream fix yet. Keep waiting.");
    process.exit(0);
  }
}

main().catch((e) => {
  console.error("ERROR:", e?.message || e);
  process.exit(1);
});
