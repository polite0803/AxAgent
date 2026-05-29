# AxAgent 版本发布流程

## 版本号规范

采用 [Semantic Versioning 2.0.0](https://semver.org/lang/zh-CN/)：

```
MAJOR.MINOR.PATCH    例: 2.4.0 → 2.5.0 → 3.0.0
```

| 位置 | 触发条件 |
|------|---------|
| **MAJOR** | Tauri 大版本升级、SeaORM breaking change、IPC 协议不兼容变更 |
| **MINOR** | 新 Tauri 命令、新功能模块、新 UI 页面、工作流节点类型新增 |
| **PATCH** | Bug 修复、性能优化、依赖安全更新、格式化/文档改动 |

预发布后缀：`-alpha.N`（内部）、`-beta.N`（外部测试）、`-rc.N`（候选发布）

## 发布节奏

- **按需发布**（非定时）：feature 累积或关键 bug 修复后发版
- **触发条件**：累计 ≥10 个 `feat`/`fix` 提交，或距上次发版 ≥2 周
- **发版负责人**在 `#release` 频道提议并获至少 1 人同意后执行

## 版本冻结窗口

| 阶段 | 时长 | 规则 |
|------|------|------|
| 🧊 **冻结期** | 发版前 24h | 只合入 `fix`/`docs`/`chore`，禁止 `feat`/`refactor` |
| 🔍 **观察期** | 发版后 48h | `latest` tag 指向最新正式版，紧急修复可上 PATCH |
| 🧪 **Beta 期** | 1 周 | 收集反馈、修复 Beta 独有 bug，稳定后升正式版 |

## 发布步骤

### 1. 确认发布内容

```bash
# 查看自上一版本以来的变更
git log v2.4.0..HEAD --oneline

# 确保 CI 全部通过
gh run list --branch master --limit 5
```

### 2. 执行版本升级

```bash
# PATCH 升级
npm run bump 2.4.1
# MINOR 升级
npm run bump 2.5.0
# Beta 预发布（手动打 tag）
npm run bump 2.5.0
git tag v2.5.0-beta.1
git push --tags
```

`npm run bump` 自动完成：
- 更新 `package.json`、`tauri.conf.json`、`Cargo.toml`
- 运行 `git-cliff` 生成 `CHANGELOG.md`
- 创建 commit + tag

### 3. 推送触发 CI

```bash
git push && git push --tags
```

推送 tag 后 GitHub Actions 自动：
1. `release.yml` 触发 → 校验版本号一致性
2. `git-cliff` 生成 Release Notes
3. 构建 macOS / Windows / Linux 安装包
4. 创建 GitHub Release（draft）

### 4. 发布确认

- [ ] CI 构建全部通过
- [ ] 下载各平台安装包进行冒烟测试
- [ ] 检查 Release Notes 内容正确
- [ ] 将 GitHub Release 从 draft 改为 published
- [ ] 通知团队

## Composer 包发布

前端 Composer 包 (`@axagent/composer`) 独立于桌面端发版：

```bash
cd packages/composer
npm version patch  # 或 minor
npm publish
```

## 回滚

如果正式版发现严重问题：

1. 基于上一版本 tag 创建 hotfix 分支
2. 合入修复 → 打 PATCH tag
3. GitHub Release 中取消旧版本 `latest` 标记
