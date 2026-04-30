# 专家功能实施总结

## 完成日期
2026-04-30

## 做了什么
在 AxAgent 项目中实现了"专家(Expert)"角色功能，取代原有的"场景(Category)"概念。

## 新增文件
| 文件 | 说明 |
|------|------|
| `src/types/expert.ts` | ExpertRole 类型 + ExpertCategory 枚举 + 分类标签映射 |
| `src/data/expertPresets.ts` | 12 个内置专家预设（通用助手/代码审查/高级开发/安全审计/数据分析/SQL专家/DevOps/技术写作/产品经理/架构师/调试专家/翻译专家） |
| `src/stores/feature/expertStore.ts` | Zustand store：角色查询、切换记录、自定义角色管理 |
| `src/components/chat/ExpertSelector.tsx` | 专家选择 Modal（2列卡片网格 + 搜索过滤，仿 WorkflowTemplateSelector 风格） |
| `src/components/chat/ExpertBadge.tsx` | 输入框上方专家徽章组件（点击弹出选择器） |

## 修改文件
| 文件 | 改动 |
|------|------|
| `src/types/index.ts` | Conversation 添加 `expert_role_id`；UpdateConversationInput 添加 `expert_role_id` |
| `src/components/chat/InputArea.tsx` | 导入 ExpertSelector/ExpertBadge；agent 模式下显示专家徽章；选择专家时更新 system_prompt 和 model 预设 |
| `src/components/chat/ChatView.tsx` | 导入 expertStore；专家切换时渲染分隔线；新增 "expert-switch" 角色渲染器 |
| `src/components/chat/ConversationSettingsModal.tsx` | system_prompt 区域显示当前专家角色 Tag |

## 关键设计决策
- 专家选择器 UI = 工作流模板选择器的孪生组件
- 专家切换 = 完全替换 system_prompt（Role Override 语义）
- 切换时消息流插入视觉分隔线（纯前端，不存库）
- Category 系统保留不动（向后兼容），前端心智模型已切换为专家
- 会话中途可随时切换专家

## 未完成项
- 侧栏按专家分类分组（ChatSidebar 改造较大，后续迭代）
- 后端 Conversation 表的 `expert_role_id` 列（需要后端配合）
