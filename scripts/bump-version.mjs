#!/usr/bin/env node
import { readFileSync, writeFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
import { execSync } from 'child_process';

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = resolve(__dirname, '..');

const args = process.argv.slice(2);
const flags = new Set(args.filter(a => a.startsWith('--')));
const positional = args.filter(a => !a.startsWith('--'));
const autoPush = flags.has('--push');

const version = positional[0];
if (!version) {
  console.error('用法: npm run bump [--push] <version>');
  console.error('示例: npm run bump 0.0.11');
  console.error('      npm run bump --push 0.0.11  (自动 push commit 和 tag)');
  process.exit(1);
}

if (!/^\d+\.\d+\.\d+/.test(version)) {
  console.error(`无效版本号: ${version}`);
  process.exit(1);
}

const files = [
  'package.json',
  'src-tauri/tauri.conf.json',
];

for (const rel of files) {
  const filepath = resolve(root, rel);
  const json = JSON.parse(readFileSync(filepath, 'utf-8'));
  const old = json.version;
  json.version = version;
  writeFileSync(filepath, JSON.stringify(json, null, 2) + '\n');
  console.log(`✅ ${rel}: ${old} → ${version}`);
}

console.log(`\n版本已更新为 ${version}`);

const tag = `v${version}`;
execSync(`git add package.json src-tauri/tauri.conf.json`, { cwd: root, stdio: 'inherit' });
execSync(`git commit -m "chore(version): bump version to ${tag}"`, { cwd: root, stdio: 'inherit' });
execSync(`git tag ${tag}`, { cwd: root, stdio: 'inherit' });
console.log(`\n🏷️  已创建 commit 和 tag: ${tag}`);

if (autoPush) {
  execSync('git push', { cwd: root, stdio: 'inherit' });
  execSync('git push --tags', { cwd: root, stdio: 'inherit' });
  console.log(`\n🚀 已推送 commit 和 tag: ${tag}`);
} else {
  console.log(`📌 执行 git push && git push --tags 即可触发 release`);
}