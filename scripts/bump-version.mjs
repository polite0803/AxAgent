#!/usr/bin/env node
import { execSync } from "child_process";
import { readFileSync, writeFileSync } from "fs";
import { dirname, resolve } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = resolve(__dirname, "..");

const args = process.argv.slice(2);
const flags = new Set(args.filter(a => a.startsWith("--")));
const positional = args.filter(a => !a.startsWith("--"));
const autoPush = flags.has("--push");

const version = positional[0];
if (!version) {
  console.error("用法: npm run bump [--push] <version>");
  console.error("示例: npm run bump 0.0.11");
  console.error("      npm run bump --push 0.0.11  (自动 push commit 和 tag)");
  process.exit(1);
}

if (!/^\d+\.\d+\.\d+/.test(version)) {
  console.error(`无效版本号: ${version}`);
  process.exit(1);
}

// --- 1. package.json ---
const pkgPath = resolve(root, "package.json");
const pkg = JSON.parse(readFileSync(pkgPath, "utf-8"));
const pkgOld = pkg.version;
pkg.version = version;
writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + "\n");
console.log(`✅ package.json: ${pkgOld} → ${version}`);

// --- 2. src-tauri/tauri.conf.json ---
const tauriConfPath = resolve(root, "src-tauri/tauri.conf.json");
const tauriConf = JSON.parse(readFileSync(tauriConfPath, "utf-8"));
const tauriOld = tauriConf.version;
tauriConf.version = version;
writeFileSync(tauriConfPath, JSON.stringify(tauriConf, null, 2) + "\n");
console.log(`✅ src-tauri/tauri.conf.json: ${tauriOld} → ${version}`);

// --- 3. src-tauri/Cargo.toml (workspace root — 子 crate 通过 workspace.package.version 自动继承) ---
const cargoPath = resolve(root, "src-tauri/Cargo.toml");
let cargoContent = readFileSync(cargoPath, "utf-8");

const workspaceVersionRegex = /^(\[workspace\.package\][\s\S]*?version\s*=\s*)"[^"]*"/;
const workspaceMatch = cargoContent.match(workspaceVersionRegex);
if (workspaceMatch) {
  const oldVersionMatch = cargoContent.match(/\[workspace\.package\][\s\S]*?version\s*=\s*"([^"]*)"/);
  const oldVersion = oldVersionMatch ? oldVersionMatch[1] : "unknown";
  cargoContent = cargoContent.replace(workspaceVersionRegex, `$1"${version}"`);

  // Also bump [package] version in the root Cargo.toml itself
  const rootPkgRegex = /^(\[package\][\s\S]*?version\s*=\s*)"[^"]*"/;
  cargoContent = cargoContent.replace(rootPkgRegex, `$1"${version}"`);

  writeFileSync(cargoPath, cargoContent);
  console.log(`✅ src-tauri/Cargo.toml (workspace.package.version): ${oldVersion} → ${version}`);
} else {
  // Fallback: just bump [package] version
  const rootPkgRegex = /^(\[package\][\s\S]*?version\s*=\s*)"[^"]*"/;
  const oldVersionMatch = cargoContent.match(/\[package\][\s\S]*?version\s*=\s*"([^"]*)"/);
  const oldVersion = oldVersionMatch ? oldVersionMatch[1] : "unknown";
  cargoContent = cargoContent.replace(rootPkgRegex, `$1"${version}"`);
  writeFileSync(cargoPath, cargoContent);
  console.log(`✅ src-tauri/Cargo.toml (package.version): ${oldVersion} → ${version}`);
}

console.log(`\n版本已更新为 ${version}`);
console.log(`ℹ️  子 crate 通过 workspace.package.version 自动继承，无需单独修改`);

// --- Git operations ---
const changedFiles = [
  "package.json",
  "src-tauri/tauri.conf.json",
  "src-tauri/Cargo.toml",
];
const gitAddFiles = changedFiles.join(" ");
const tag = `v${version}`;
execSync(`git add ${gitAddFiles}`, { cwd: root, stdio: "inherit" });
execSync(`git commit -m "chore(version): bump version to ${tag}"`, { cwd: root, stdio: "inherit" });
execSync(`git tag ${tag}`, { cwd: root, stdio: "inherit" });
console.log(`\n🏷️  已创建 commit 和 tag: ${tag}`);

if (autoPush) {
  execSync("git push", { cwd: root, stdio: "inherit" });
  execSync("git push --tags", { cwd: root, stdio: "inherit" });
  console.log(`\n🚀 已推送 commit 和 tag: ${tag}`);
} else {
  console.log(`📌 执行 git push && git push --tags 即可触发 release`);
}
