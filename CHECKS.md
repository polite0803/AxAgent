# 发版检查清单

> 每次发布新版本前，按顺序执行以下检查。

## Rust 后端

```bash
# 1. 格式化检查
cd src-tauri && cargo fmt --all -- --check

# 2. Clippy 零警告
cargo clippy --workspace --all-features -- -D warnings

# 3. 编译检查
cargo check --workspace --all-features

# 4. 运行所有测试
cargo test --workspace --all-features

# 5. 覆盖率检查（若有变更）
cargo llvm-cov --workspace --all-features
```

## 前端

```bash
# 6. 格式化检查（项目根目录）
npm run format:check

# 7. TypeScript 类型检查
npm run typecheck

# 8. ESLint 零警告
npx eslint src --max-warnings=0

# 9. 前端测试
npm run test:run

# 10. 生产构建
npm run build
```

## 安全审计

```bash
# 11. Rust 依赖漏洞扫描
cd src-tauri && cargo audit

# 12. npm 依赖审计
npm audit --audit-level=high
```

## 发布前确认

- [ ] 以上 12 步全部通过
- [ ] `CHANGELOG.md` 已更新
- [ ] 版本号已升级（`npm run bump`）
- [ ] README 中的版本/数据已同步
