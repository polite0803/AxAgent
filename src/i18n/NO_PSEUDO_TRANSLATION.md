# NO PSEUDO TRANSLATION ALLOWED

## 规则
- 任何新增 i18n key **必须** 走真实翻译流程：先 en 添加 → 然后人工/翻译服务补 12 语言
- **禁止** 跑 `compare_locales.js` 等自动脚本生成假翻译（英文复制到其他语言文件）
- 任何 i18n drift 必须在 PR 中修复，不能积累

## 历史问题
- 2026-06-11 代码审查发现 `ja.json` 出现 1229 行 en-US 没有的 key
- 这些 key 是历史 `compare_locales.js` 自动生成流程的副作用
- 将在 Task 1.4 中清理并迁移到 `docs/baseline/ja-extra-keys.txt`

## CI 校验
- `scripts/i18n-check.mjs`（Task 1.4 引入）会在 CI 阻止任何 missing/extra key
- `scripts/ci-check.sh` 第 5 步自动运行 i18n 校验
