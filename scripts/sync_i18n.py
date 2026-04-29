#!/usr/bin/env python3
"""
Sync i18n locale files: add all missing keys used in code to every locale file.
- zh-CN: Chinese translations
- en-US: English translations
- Other locales: use en-US values as fallback
"""

import json
import os
import copy

LOCALES_DIR = os.path.join(os.path.dirname(__file__), '..', 'src', 'i18n', 'locales')

# ============================================================
# Missing translations for zh-CN
# ============================================================
ZH_MISSING = {
  "agent": {
    "answerSubmitted": "回答已提交",
    "pleaseEnterAnswer": "请输入回答",
    "pleaseSelectOption": "请选择一个选项",
    "questionFromAgent": "Agent 的问题",
    "selectOneOption": "选择一个选项",
    "selectOptions": "选择选项",
    "submitAnswer": "提交回答",
    "submitting": "提交中...",
    "supplementPlaceholder": "补充说明（可选）...",
    "typeAnswerPlaceholder": "输入您的回答..."
  },
  "browser": {
    "clickBody": "点击页面主体",
    "clickSuccess": "点击成功",
    "closed": "浏览器已关闭",
    "elementsFound": "找到 {{count}} 个元素",
    "extractElements": "提取元素",
    "fillSuccess": "填充成功",
    "fillText": "填充文本",
    "navigateSuccess": "导航成功",
    "pleaseEnterSelector": "请输入选择器",
    "quickActions": "快捷操作",
    "screenshotSuccess": "截图成功"
  },
  "chat": {
    "agentStats": {
      "pause": "暂停",
      "paused": "已暂停",
      "pending": "待处理",
      "resume": "继续",
      "running": "运行中",
      "session": "个会话",
      "tokens": "个令牌",
      "tool": "个工具"
    },
    "benchmarks": {
      "failed": "失败",
      "passed": "通过",
      "refresh": "刷新",
      "results": "结果",
      "run": "运行",
      "tasks": "任务",
      "title": "基准测试"
    },
    "categorySystemPromptPlaceholder": "输入分类系统提示词...",
    "chart": {
      "analysis": "分析",
      "analyzing": "分析中...",
      "avg": "平均",
      "chartImage": "图表图片",
      "dataPoints": "数据点",
      "insights": "洞察",
      "label": "标签",
      "max": "最大",
      "min": "最小",
      "noAnalysis": "暂无分析",
      "rawData": "原始数据",
      "series": "系列",
      "summary": "摘要",
      "value": "值"
    },
    "collaboration": {
      "copied": "已复制",
      "copyCode": "复制代码",
      "noSessions": "暂无协作会话",
      "participants": "参与者",
      "session": "会话",
      "sessionShare": {
        "copied": "已复制",
        "copy": "复制",
        "enterInviteCode": "输入邀请码",
        "fileAccess": "文件访问",
        "inviteCode": "邀请码",
        "joinMode": "加入模式",
        "joinSession": "加入会话",
        "modelAccess": "模型访问",
        "permissions": "权限",
        "requireApproval": "需要审批",
        "shareMode": "分享模式",
        "terminalAccess": "终端访问",
        "title": "会话共享"
      },
      "sharedResources": "共享资源",
      "title": "协作"
    },
    "conversationNotFound": "对话未找到",
    "copyMarkdown": "复制 Markdown",
    "exportJsonNoThinking": "导出 JSON（不含思考）",
    "exportMdNoThinking": "导出 Markdown（不含思考）",
    "exportTxtNoThinking": "导出文本（不含思考）",
    "git": {
      "branchComparison": "分支对比",
      "commitError": "提交失败",
      "commitMessage": "提交信息",
      "commitSuccess": "提交成功",
      "commits": "提交记录",
      "createCommit": "创建提交",
      "emptyMessage": "提交信息不能为空",
      "filesChanged": "变更文件",
      "generateMessage": "生成提交信息",
      "generateMessageError": "生成提交信息失败",
      "generatedSuggestion": "已生成建议",
      "messagePlaceholder": "输入提交信息...",
      "noRepo": "未找到 Git 仓库",
      "noStagedChanges": "没有暂存的更改",
      "prDescription": "PR 描述",
      "stagedChanges": "暂存更改",
      "title": "Git 操作"
    },
    "inspector": {
      "toolCalls": "工具调用",
      "toolError": "工具错误"
    },
    "plugins": {
      "marketplace": {
        "disable": "禁用",
        "enable": "启用",
        "install": "安装",
        "loading": "加载中...",
        "refresh": "刷新",
        "title": "插件市场"
      }
    },
    "rewardWeights": "奖励权重",
    "steer": "引导",
    "steerPlaceholder": "输入引导指令...",
    "totalTokens": "总 Tokens",
    "vision": {
      "analyzedImage": "已分析图片",
      "analyzing": "分析中...",
      "description": "描述",
      "elements": "元素",
      "extractedText": "提取文本",
      "imageAnalysis": "图片分析",
      "model": "模型",
      "ui": {
        "actionable": "可操作",
        "analyzing": "分析中...",
        "clickable": "可点击",
        "description": "描述",
        "elementList": "元素列表",
        "elements": "元素",
        "noElements": "未找到元素",
        "screenshot": "截图",
        "static": "静态",
        "title": "UI 分析",
        "types": "类型"
      }
    },
    "waiting": "等待中..."
  },
  "common": {
    "add": "添加",
    "affectedPaths": "受影响路径",
    "clear": "清除",
    "confirmExecute": "确认执行",
    "dangerousOperationWarning": "⚠️ 此操作可能存在风险，请谨慎执行。",
    "download": "下载",
    "executionWarning": "执行警告",
    "learnAboutPermissions": "了解权限",
    "postedOn": "发布于",
    "tags": "标签",
    "text": "文本",
    "viewDetails": "查看详情",
    "waitingForApproval": "等待审批"
  },
  "link": {
    "actions": "操作",
    "noModelSelected": "未选择模型",
    "noSkillSelected": "未选择技能",
    "pushSelected": "推送选中",
    "syncAllSkillsFailed": "同步所有技能失败",
    "syncAllSkillsSuccess": "同步所有技能成功",
    "updateSettingsFailed": "更新设置失败"
  },
  "marketplace": {
    "author": "作者",
    "categories": "分类",
    "customerReviews": "用户评价",
    "details": "详情",
    "downloads": "下载量",
    "importWorkflow": "导入工作流",
    "noTemplatesFound": "未找到模板",
    "quickActions": "快捷操作",
    "reviews": "评价",
    "title": "市场",
    "writeReview": "撰写评价",
    "yourReview": "我的评价"
  },
  "notification": {
    "clear": "清除",
    "empty": "暂无通知",
    "hoursAgo": "{{count}} 小时前",
    "justNow": "刚刚",
    "markAllRead": "全部标记已读",
    "minutesAgo": "{{count}} 分钟前",
    "title": "通知"
  },
  "nudge": {
    "acceptanceRate": "接受率",
    "dismiss": "忽略",
    "execute": "执行",
    "learningSuggestions": "学习建议",
    "snooze30": "稍后提醒（30分钟）"
  },
  "proactive": {
    "accept": "接受",
    "add": "添加",
    "addReminder": "添加提醒",
    "cancel": "取消",
    "complete": "完成",
    "completed": "已完成",
    "contextPrediction": "上下文预测",
    "delete": "删除",
    "description": "描述",
    "descriptionPlaceholder": "输入描述...",
    "dismiss": "忽略",
    "dismissAll": "全部忽略",
    "estimatedTime": "预计时间",
    "inDays": "{{count}} 天后",
    "language": "语言",
    "loading": "加载中...",
    "moreSuggestions": "更多建议",
    "noPredictions": "暂无预测",
    "noReminders": "暂无提醒",
    "prefetchReady": "预取就绪",
    "prefetching": "预取中...",
    "reminders": "提醒",
    "scheduledAt": "计划时间",
    "snooze": "稍后提醒",
    "suggestedActions": "建议操作",
    "suggestions": "建议",
    "title": "主动助手",
    "titlePlaceholder": "输入标题...",
    "today": "今天",
    "tomorrow": "明天",
    "unknown": "未知",
    "yesterday": "昨天"
  },
  "promptTemplates": {
    "content": "内容",
    "deleteTemplate": "删除模板",
    "deleteTemplateConfirm": "确定删除此模板？",
    "descriptionPlaceholder": "输入描述...",
    "editTemplate": "编辑模板",
    "templateCreated": "模板已创建",
    "templateDeleted": "模板已删除",
    "templateUpdated": "模板已更新",
    "variablesSchema": "变量 Schema",
    "version": "版本"
  },
  "review": {
    "commentPlaceholder": "输入评论...",
    "deletedSuccess": "删除成功",
    "failedToDelete": "删除失败",
    "failedToSubmit": "提交失败",
    "ratingRequired": "请选择评分",
    "submittedSuccess": "提交成功",
    "updatedSuccess": "更新成功"
  },
  "search": {
    "caseSensitive": "区分大小写",
    "clearRecent": "清除最近",
    "filterName": "筛选器名称",
    "filters": "筛选器",
    "limit": "限制",
    "noResults": "无结果",
    "placeholder": "搜索...",
    "recentSearches": "最近搜索",
    "saveFilter": "保存筛选器",
    "saveFilterTitle": "保存筛选器",
    "savedFilters": "已保存筛选器",
    "title": "搜索",
    "useRegex": "使用正则表达式"
  },
  "settings": {
    "activeProfile": "当前配置",
    "createProfile": "创建配置",
    "disabledProviders": "已禁用的服务商",
    "enabledProviders": "已启用的服务商",
    "newProfile": "新建配置",
    "profileDisplayName": "配置显示名称",
    "profileManager": "配置管理",
    "profileName": "配置名称",
    "profiles": "配置",
    "shortcuts": {
      "description": "自定义快捷键绑定",
      "reset": "重置",
      "resetAll": "重置所有",
      "saved": "已保存",
      "title": "快捷键"
    },
    "theme": {
      "builtInThemes": "内置主题",
      "customThemes": "自定义主题",
      "deleteConfirm": "确定删除此主题？",
      "deleted": "主题已删除",
      "description": "管理应用主题和颜色方案",
      "exported": "主题已导出",
      "import": "导入",
      "importTitle": "导入主题",
      "imported": "主题已导入",
      "invalidYaml": "无效的 YAML 格式",
      "refresh": "刷新",
      "title": "主题管理",
      "yamlContent": "YAML 内容",
      "yamlRequired": "YAML 内容不能为空"
    }
  },
  "skill": {
    "content": "技能内容",
    "contentPlaceholder": "输入技能内容...",
    "contentRequired": "请输入技能内容",
    "create": "创建",
    "createTitle": "创建新技能",
    "description": "描述",
    "descriptionPlaceholder": "输入描述...",
    "edit": "编辑",
    "editPlaceholder": "编写完整的技能内容...",
    "editTitle": "编辑技能",
    "name": "名称",
    "namePlaceholder": "输入名称...",
    "nameRequired": "请输入名称",
    "patch": "修补",
    "patchPlaceholder": "描述问题和修复方法...",
    "patchTitle": "修补技能",
    "proposal": {
      "confidence": "置信度",
      "create": "创建",
      "created": "技能已创建",
      "dismiss": "忽略",
      "empty": "暂无技能提议",
      "error": "创建失败",
      "failedWorkflow": "失败工作流",
      "hint": "这些技能是从成功的 Agent 工作流自动生成的",
      "partialSuccess": "部分成功",
      "successWorkflow": "成功工作流",
      "title": "技能提议",
      "viewContent": "查看内容"
    }
  },
  "skills": {
    "atomicSkills": "原子技能"
  },
  "style": {
    "adjustments": "调整",
    "codeSamples": "代码示例",
    "descriptions": {
      "abstraction": "抽象程度：从具体实现到抽象概念的跨度",
      "commentRatio": "注释比率：代码中注释的密度",
      "density": "信息密度：输出的紧凑程度",
      "explanationLength": "解释长度：说明的详尽程度",
      "formality": "正式程度：语言的正式与随意程度",
      "naming": "命名风格：变量和函数的命名约定",
      "structure": "结构化程度：内容组织的条理性",
      "technicalDepth": "技术深度：技术细节的深入程度"
    },
    "dimensions": {
      "abstraction": "抽象程度",
      "commentRatio": "注释比率",
      "density": "信息密度",
      "explanationLength": "解释长度",
      "formality": "正式程度",
      "naming": "命名风格",
      "structure": "结构化程度",
      "technicalDepth": "技术深度"
    },
    "dimensionsCount": "{{count}} 个维度",
    "labels": {
      "abstract": "抽象",
      "advanced": "高级",
      "basic": "基础",
      "brief": "简短",
      "camelCase": "驼峰命名",
      "casual": "随意",
      "compact": "紧凑",
      "comprehensive": "全面",
      "concrete": "具体",
      "detailed": "详细",
      "formal": "正式",
      "minimal": "极简",
      "simple": "简单",
      "snakeCase": "蛇形命名",
      "spacious": "宽松",
      "structured": "结构化"
    },
    "noPatterns": "暂无风格模式",
    "noTemplates": "暂无风格模板",
    "patterns": "模式",
    "presets": {
      "maximal": "最大化",
      "minimal": "最小化",
      "neutral": "中性"
    },
    "showMorePatterns": "显示更多模式",
    "showMoreTemplates": "显示更多模板",
    "sourceStyle": "源风格",
    "targetStyle": "目标风格",
    "templates": "模板",
    "usedCount": "使用 {{count}} 次",
    "variants": "变体"
  },
  "wiki": {
    "common": {
      "actions": "操作",
      "back": "返回",
      "close": "关闭",
      "overview": "概览",
      "refresh": "刷新"
    },
    "contentPlaceholder": "输入内容...",
    "emptyNotes": "暂无笔记",
    "graph": {
      "edges": "边",
      "empty": "暂无图谱数据",
      "filterByTags": "按标签筛选",
      "filterByType": "按类型筛选",
      "filters": "筛选器",
      "loadError": "加载失败",
      "loading": "加载中...",
      "noWikiId": "未指定 Wiki ID",
      "nodes": "节点",
      "stats": "统计",
      "title": "知识图谱",
      "type": {
        "concept": "概念",
        "entity": "实体",
        "note": "笔记",
        "source": "来源"
      }
    },
    "ingest": {
      "docx": "Word 文档",
      "file": "文件",
      "folder": "文件夹",
      "folderPath": "文件夹路径",
      "folderPathPlaceholder": "输入文件夹路径...",
      "folderPathRequired": "请输入文件夹路径",
      "history": "导入历史",
      "markdown": "Markdown",
      "notion": "Notion",
      "path": "路径",
      "pathPlaceholder": "输入路径...",
      "pdf": "PDF",
      "results": "导入结果",
      "sourceType": "来源类型",
      "sourceTypeRequired": "请选择来源类型",
      "start": "开始导入",
      "title": "导入来源",
      "titlePlaceholder": "输入标题...",
      "type": "类型",
      "upload": "上传",
      "uploadFile": "上传文件",
      "uploadHint": "拖放文件或点击上传",
      "url": "URL",
      "urlInvalid": "URL 格式无效",
      "urlRequired": "请输入 URL",
      "web": "网页"
    },
    "insertLink": "插入链接",
    "lastCompiled": "上次编译",
    "lastLinted": "上次检查",
    "lint": {
      "averageScore": "平均分",
      "code": "代码",
      "details": "详情",
      "issues": "问题",
      "line": "行",
      "loadError": "加载失败",
      "message": "消息",
      "noIssues": "无问题",
      "noResults": "无结果",
      "note": "笔记",
      "rerun": "重新运行",
      "runLint": "运行检查",
      "score": "分数",
      "scoreUpdated": "分数已更新",
      "selectNote": "选择笔记",
      "severity": "严重程度",
      "success": "检查完成",
      "totalIssues": "总计问题",
      "totalNotes": "总计笔记",
      "updateScore": "更新分数",
      "viewDetails": "查看详情"
    },
    "llm": {
      "backToList": "返回列表",
      "compile": "编译",
      "compileErrors": "编译错误",
      "compileSuccess": "编译成功",
      "confirmDelete": "确定删除？",
      "create": "创建",
      "createSuccess": "创建成功",
      "createWiki": "创建 Wiki",
      "deleteSuccess": "删除成功",
      "descriptionPlaceholder": "输入描述...",
      "fileUploaded": "文件已上传",
      "ingestError": "导入失败",
      "ingestSource": "导入来源",
      "ingestSuccess": "导入成功",
      "lintReport": "检查报告",
      "lowQualityWarning": "质量较低，建议改进",
      "namePlaceholder": "输入名称...",
      "nameRequired": "请输入名称",
      "newCompiledContent": "新编译内容",
      "noScore": "暂无评分",
      "operations": "操作",
      "pathPlaceholder": "输入路径...",
      "pathRequired": "请输入路径",
      "readyToSave": "可以保存",
      "recompile": "重新编译",
      "select": "选择",
      "selectSourcesFirst": "请先选择来源",
      "sources": "来源",
      "stats": {
        "lastCompile": "上次编译",
        "operations": "操作数",
        "sources": "来源数",
        "totalWikis": "Wiki 总数"
      },
      "title": "Wiki 管理",
      "uploadError": "上传失败",
      "viewInGraph": "在图谱中查看",
      "viewSource": "查看来源",
      "wikiList": "Wiki 列表"
    },
    "llmNote": "LLM 笔记",
    "log": {
      "completed": "已完成",
      "failed": "失败",
      "filterStatus": "按状态筛选",
      "filterType": "按类型筛选",
      "noOperations": "暂无操作记录",
      "operationDetail": "操作详情",
      "running": "运行中",
      "title": "操作日志",
      "total": "总计"
    },
    "noSources": "暂无来源",
    "noteNotFound": "笔记未找到",
    "notes": "笔记",
    "operation": {
      "compile": "编译",
      "completedAt": "完成时间",
      "createdAt": "创建时间",
      "details": "详情",
      "duration": "耗时",
      "error": "错误",
      "id": "ID",
      "ingest": "导入",
      "lint": "检查",
      "noOperations": "暂无操作",
      "page": "页面",
      "result": "结果",
      "schema": "Schema",
      "source": "来源",
      "status": "状态",
      "sync": "同步",
      "type": "类型"
    },
    "pageNotFound": "页面未找到",
    "pageType": "页面类型",
    "quality": {
      "excellent": "优秀",
      "factors": "质量因素",
      "fair": "一般",
      "good": "良好",
      "issueList": "问题列表",
      "issues": "问题",
      "more": "更多",
      "noData": "暂无数据",
      "poor": "较差",
      "refresh": "刷新",
      "title": "质量评估"
    },
    "qualityScore": "质量分数",
    "save": "保存",
    "saved": "已保存",
    "schema": {
      "create": "创建",
      "created": "已创建",
      "createdAt": "创建时间",
      "deleteConfirm": "确定删除？",
      "deleted": "已删除",
      "description": "描述",
      "descriptionPlaceholder": "输入描述...",
      "edit": "编辑",
      "noSchemas": "暂无 Schema",
      "schema": "Schema",
      "schemaHelp": "定义 Wiki 的数据结构",
      "title": "Schema 管理",
      "updated": "已更新",
      "version": "版本"
    },
    "searchPlaceholder": "搜索 Wiki...",
    "selectWiki": "选择 Wiki",
    "selectWikiPrompt": "请选择一个 Wiki",
    "source": {
      "chunks": "分段",
      "deleteNotImplemented": "删除功能尚未实现",
      "path": "路径",
      "status": "状态",
      "title": "来源",
      "type": "类型"
    },
    "sources": "来源",
    "sync": {
      "byWiki": "按 Wiki",
      "capacity": "容量",
      "emptyQueue": "队列为空",
      "failed": "同步失败",
      "linkCreated": "链接已创建",
      "linkDeleted": "链接已删除",
      "more": "更多",
      "noteCreated": "笔记已创建",
      "noteDeleted": "笔记已删除",
      "noteUpdated": "笔记已更新",
      "pending": "待处理",
      "processNow": "立即处理",
      "processStarted": "处理已开始",
      "processing": "处理中...",
      "queue": "队列",
      "refresh": "刷新",
      "retryCount": "重试次数",
      "title": "同步",
      "usage": "使用量"
    },
    "titlePlaceholder": "输入标题...",
    "userNote": "用户笔记",
    "wiki": {
      "description": "描述",
      "name": "名称",
      "rootPath": "根路径",
      "schemaVersion": "Schema 版本"
    }
  },
  "workEngine": {
    "cancel": "取消",
    "cancelExecution": "取消执行",
    "confirmCancelExecution": "确定取消当前执行？",
    "duration": "耗时",
    "executionHistory": "执行历史",
    "executionId": "执行 ID",
    "pause": "暂停",
    "pauseExecution": "暂停执行",
    "resume": "继续",
    "resumeExecution": "继续执行",
    "start": "开始",
    "startExecution": "开始执行",
    "startTime": "开始时间",
    "status": "状态",
    "statusCancelled": "已取消",
    "statusCompleted": "已完成",
    "statusFailed": "失败",
    "statusPaused": "已暂停",
    "statusPending": "待执行",
    "statusRunning": "运行中"
  },
  "workflow": {
    "applySemanticCheck": "应用",
    "applyUpgrade": "应用升级",
    "category": "分类",
    "decompositionSaved": "分解已保存",
    "description": "描述",
    "entryType": "条目类型",
    "existingSkill": "现有技能",
    "generatedSkill": "生成技能",
    "generatingUpgrade": "正在生成升级建议...",
    "inputSchema": "输入 Schema",
    "keepGeneratedSkill": "保留生成",
    "matchReasons": "匹配原因",
    "outputSchema": "输出 Schema",
    "replaceExisting": "替换现有",
    "replaceWithExisting": "使用现有",
    "saveAsNew": "另存为新",
    "saved": "已保存",
    "semanticCheckApplied": "语义检查已应用",
    "semanticCheckDescription": "以下生成的技能与现有技能具有高语义相似度",
    "semanticCheckTitle": "发现相似技能",
    "similarWorkflowsExplanation": "发现与当前工作流相似的工作流",
    "similarWorkflowsFound": "找到相似工作流",
    "similarity": "相似度",
    "skills": "技能",
    "skipSemanticCheck": "跳过",
    "upgradeError": "升级失败",
    "upgradeExisting": "升级现有",
    "upgradeReasoning": "升级理由",
    "upgradeSkillTitle": "升级技能",
    "upgradeSuggestion": "升级建议",
    "upgradedName": "升级后名称",
    "validationFailed": "验证失败",
    "workflowSavedAsNew": "工作流已另存为新",
    "workflowUpdated": "工作流已更新"
  }
}

# ============================================================
# Missing translations for en-US
# ============================================================
EN_MISSING = {
  "agent": {
    "answerSubmitted": "Answer submitted",
    "pleaseEnterAnswer": "Please enter your answer",
    "pleaseSelectOption": "Please select an option",
    "questionFromAgent": "Question from Agent",
    "selectOneOption": "Select one option",
    "selectOptions": "Select options",
    "submitAnswer": "Submit answer",
    "submitting": "Submitting...",
    "supplementPlaceholder": "Additional notes (optional)...",
    "typeAnswerPlaceholder": "Type your answer..."
  },
  "browser": {
    "clickBody": "Click page body",
    "clickSuccess": "Click successful",
    "closed": "Browser closed",
    "elementsFound": "{{count}} elements found",
    "extractElements": "Extract elements",
    "fillSuccess": "Fill successful",
    "fillText": "Fill text",
    "navigateSuccess": "Navigate successful",
    "pleaseEnterSelector": "Please enter a selector",
    "quickActions": "Quick actions",
    "screenshotSuccess": "Screenshot successful"
  },
  "chat": {
    "agentStats": {
      "pause": "Pause",
      "paused": "Paused",
      "pending": "Pending",
      "resume": "Resume",
      "running": "Running",
      "session": "sessions",
      "tokens": "tokens",
      "tool": "tools"
    },
    "benchmarks": {
      "failed": "Failed",
      "passed": "Passed",
      "refresh": "Refresh",
      "results": "Results",
      "run": "Run",
      "tasks": "Tasks",
      "title": "Benchmarks"
    },
    "categorySystemPromptPlaceholder": "Enter category system prompt...",
    "chart": {
      "analysis": "Analysis",
      "analyzing": "Analyzing...",
      "avg": "Average",
      "chartImage": "Chart Image",
      "dataPoints": "Data Points",
      "insights": "Insights",
      "label": "Label",
      "max": "Max",
      "min": "Min",
      "noAnalysis": "No analysis available",
      "rawData": "Raw Data",
      "series": "Series",
      "summary": "Summary",
      "value": "Value"
    },
    "collaboration": {
      "copied": "Copied",
      "copyCode": "Copy code",
      "noSessions": "No collaboration sessions",
      "participants": "Participants",
      "session": "Session",
      "sessionShare": {
        "copied": "Copied",
        "copy": "Copy",
        "enterInviteCode": "Enter invite code",
        "fileAccess": "File Access",
        "inviteCode": "Invite Code",
        "joinMode": "Join Mode",
        "joinSession": "Join Session",
        "modelAccess": "Model Access",
        "permissions": "Permissions",
        "requireApproval": "Require Approval",
        "shareMode": "Share Mode",
        "terminalAccess": "Terminal Access",
        "title": "Session Sharing"
      },
      "sharedResources": "Shared Resources",
      "title": "Collaboration"
    },
    "conversationNotFound": "Conversation not found",
    "copyMarkdown": "Copy Markdown",
    "exportJsonNoThinking": "Export JSON (no thinking)",
    "exportMdNoThinking": "Export Markdown (no thinking)",
    "exportTxtNoThinking": "Export Text (no thinking)",
    "git": {
      "branchComparison": "Branch Comparison",
      "commitError": "Commit failed",
      "commitMessage": "Commit message",
      "commitSuccess": "Committed successfully",
      "commits": "Commits",
      "createCommit": "Create Commit",
      "emptyMessage": "Commit message cannot be empty",
      "filesChanged": "Files changed",
      "generateMessage": "Generate message",
      "generateMessageError": "Failed to generate message",
      "generatedSuggestion": "Suggestion generated",
      "messagePlaceholder": "Enter commit message...",
      "noRepo": "No Git repository found",
      "noStagedChanges": "No staged changes",
      "prDescription": "PR Description",
      "stagedChanges": "Staged Changes",
      "title": "Git Operations"
    },
    "inspector": {
      "toolCalls": "Tool Calls",
      "toolError": "Tool Error"
    },
    "plugins": {
      "marketplace": {
        "disable": "Disable",
        "enable": "Enable",
        "install": "Install",
        "loading": "Loading...",
        "refresh": "Refresh",
        "title": "Plugin Marketplace"
      }
    },
    "rewardWeights": "Reward Weights",
    "steer": "Steer",
    "steerPlaceholder": "Enter steering instruction...",
    "totalTokens": "Total Tokens",
    "vision": {
      "analyzedImage": "Analyzed image",
      "analyzing": "Analyzing...",
      "description": "Description",
      "elements": "Elements",
      "extractedText": "Extracted text",
      "imageAnalysis": "Image Analysis",
      "model": "Model",
      "ui": {
        "actionable": "Actionable",
        "analyzing": "Analyzing...",
        "clickable": "Clickable",
        "description": "Description",
        "elementList": "Element List",
        "elements": "Elements",
        "noElements": "No elements found",
        "screenshot": "Screenshot",
        "static": "Static",
        "title": "UI Analysis",
        "types": "Types"
      }
    },
    "waiting": "Waiting..."
  },
  "common": {
    "add": "Add",
    "affectedPaths": "Affected paths",
    "clear": "Clear",
    "confirmExecute": "Confirm execute",
    "dangerousOperationWarning": "⚠️ This operation may be risky. Proceed with caution.",
    "download": "Download",
    "executionWarning": "Execution Warning",
    "learnAboutPermissions": "Learn about permissions",
    "postedOn": "Posted on",
    "tags": "Tags",
    "text": "Text",
    "viewDetails": "View details",
    "waitingForApproval": "Waiting for approval"
  },
  "link": {
    "actions": "Actions",
    "noModelSelected": "No model selected",
    "noSkillSelected": "No skill selected",
    "pushSelected": "Push selected",
    "syncAllSkillsFailed": "Sync all skills failed",
    "syncAllSkillsSuccess": "Sync all skills successful",
    "updateSettingsFailed": "Update settings failed"
  },
  "marketplace": {
    "author": "Author",
    "categories": "Categories",
    "customerReviews": "Customer Reviews",
    "details": "Details",
    "downloads": "Downloads",
    "importWorkflow": "Import Workflow",
    "noTemplatesFound": "No templates found",
    "quickActions": "Quick Actions",
    "reviews": "Reviews",
    "title": "Marketplace",
    "writeReview": "Write Review",
    "yourReview": "Your Review"
  },
  "notification": {
    "clear": "Clear",
    "empty": "No notifications",
    "hoursAgo": "{{count}}h ago",
    "justNow": "Just now",
    "markAllRead": "Mark all read",
    "minutesAgo": "{{count}}m ago",
    "title": "Notifications"
  },
  "nudge": {
    "acceptanceRate": "Acceptance rate",
    "dismiss": "Dismiss",
    "execute": "Execute",
    "learningSuggestions": "Learning Suggestions",
    "snooze30": "Snooze (30 min)"
  },
  "proactive": {
    "accept": "Accept",
    "add": "Add",
    "addReminder": "Add Reminder",
    "cancel": "Cancel",
    "complete": "Complete",
    "completed": "Completed",
    "contextPrediction": "Context Prediction",
    "delete": "Delete",
    "description": "Description",
    "descriptionPlaceholder": "Enter description...",
    "dismiss": "Dismiss",
    "dismissAll": "Dismiss All",
    "estimatedTime": "Estimated Time",
    "inDays": "In {{count}} days",
    "language": "Language",
    "loading": "Loading...",
    "moreSuggestions": "More Suggestions",
    "noPredictions": "No predictions",
    "noReminders": "No reminders",
    "prefetchReady": "Prefetch ready",
    "prefetching": "Prefetching...",
    "reminders": "Reminders",
    "scheduledAt": "Scheduled at",
    "snooze": "Snooze",
    "suggestedActions": "Suggested Actions",
    "suggestions": "Suggestions",
    "title": "Proactive Assistant",
    "titlePlaceholder": "Enter title...",
    "today": "Today",
    "tomorrow": "Tomorrow",
    "unknown": "Unknown",
    "yesterday": "Yesterday"
  },
  "promptTemplates": {
    "content": "Content",
    "deleteTemplate": "Delete Template",
    "deleteTemplateConfirm": "Delete this template?",
    "descriptionPlaceholder": "Enter description...",
    "editTemplate": "Edit Template",
    "templateCreated": "Template created",
    "templateDeleted": "Template deleted",
    "templateUpdated": "Template updated",
    "variablesSchema": "Variables Schema",
    "version": "Version"
  },
  "review": {
    "commentPlaceholder": "Enter comment...",
    "deletedSuccess": "Deleted successfully",
    "failedToDelete": "Failed to delete",
    "failedToSubmit": "Failed to submit",
    "ratingRequired": "Rating is required",
    "submittedSuccess": "Submitted successfully",
    "updatedSuccess": "Updated successfully"
  },
  "search": {
    "caseSensitive": "Case sensitive",
    "clearRecent": "Clear recent",
    "filterName": "Filter name",
    "filters": "Filters",
    "limit": "Limit",
    "noResults": "No results",
    "placeholder": "Search...",
    "recentSearches": "Recent searches",
    "saveFilter": "Save filter",
    "saveFilterTitle": "Save Filter",
    "savedFilters": "Saved filters",
    "title": "Search",
    "useRegex": "Use regex"
  },
  "settings": {
    "activeProfile": "Active Profile",
    "createProfile": "Create Profile",
    "disabledProviders": "Disabled Providers",
    "enabledProviders": "Enabled Providers",
    "newProfile": "New Profile",
    "profileDisplayName": "Profile Display Name",
    "profileManager": "Profile Manager",
    "profileName": "Profile Name",
    "profiles": "Profiles",
    "shortcuts": {
      "description": "Customize keyboard shortcut bindings",
      "reset": "Reset",
      "resetAll": "Reset All",
      "saved": "Saved",
      "title": "Shortcuts"
    },
    "theme": {
      "builtInThemes": "Built-in Themes",
      "customThemes": "Custom Themes",
      "deleteConfirm": "Delete this theme?",
      "deleted": "Theme deleted",
      "description": "Manage application themes and color schemes",
      "exported": "Theme exported",
      "import": "Import",
      "importTitle": "Import Theme",
      "imported": "Theme imported",
      "invalidYaml": "Invalid YAML format",
      "refresh": "Refresh",
      "title": "Theme Manager",
      "yamlContent": "YAML Content",
      "yamlRequired": "YAML content is required"
    },
    "workflow": {
      "title": "Workflow Settings",
      "description": "Manage workflow templates and editor settings",
      "createNew": "Create New Template",
      "visualEditor": "Visual Editor",
      "visualEditorDesc": "Create and edit workflows with drag-and-drop DAG editor",
      "openEditor": "Open Editor"
    }
  },
  "skill": {
    "content": "Content",
    "contentPlaceholder": "Enter skill content...",
    "contentRequired": "Content is required",
    "create": "Create",
    "createTitle": "Create New Skill",
    "description": "Description",
    "descriptionPlaceholder": "Enter description...",
    "edit": "Edit",
    "editPlaceholder": "Write the full skill content...",
    "editTitle": "Edit Skill",
    "name": "Name",
    "namePlaceholder": "Enter name...",
    "nameRequired": "Name is required",
    "patch": "Patch",
    "patchPlaceholder": "Describe the issue and fix...",
    "patchTitle": "Patch Skill",
    "proposal": {
      "confidence": "Confidence",
      "create": "Create",
      "created": "Skill created",
      "dismiss": "Dismiss",
      "empty": "No skill proposals",
      "error": "Creation failed",
      "failedWorkflow": "Failed workflow",
      "hint": "These skills are auto-generated from successful Agent workflows",
      "partialSuccess": "Partial success",
      "successWorkflow": "Successful workflow",
      "title": "Skill Proposal",
      "viewContent": "View content"
    }
  },
  "skills": {
    "atomicSkills": "Atomic Skills"
  },
  "style": {
    "adjustments": "Adjustments",
    "codeSamples": "Code Samples",
    "descriptions": {
      "abstraction": "Abstraction: from concrete to abstract concepts",
      "commentRatio": "Comment Ratio: density of comments in code",
      "density": "Information Density: compactness of output",
      "explanationLength": "Explanation Length: thoroughness of explanations",
      "formality": "Formality: formal vs casual language",
      "naming": "Naming: variable and function naming conventions",
      "structure": "Structure: organization of content",
      "technicalDepth": "Technical Depth: level of technical detail"
    },
    "dimensions": {
      "abstraction": "Abstraction",
      "commentRatio": "Comment Ratio",
      "density": "Density",
      "explanationLength": "Explanation Length",
      "formality": "Formality",
      "naming": "Naming",
      "structure": "Structure",
      "technicalDepth": "Technical Depth"
    },
    "dimensionsCount": "{{count}} dimensions",
    "labels": {
      "abstract": "Abstract",
      "advanced": "Advanced",
      "basic": "Basic",
      "brief": "Brief",
      "camelCase": "camelCase",
      "casual": "Casual",
      "compact": "Compact",
      "comprehensive": "Comprehensive",
      "concrete": "Concrete",
      "detailed": "Detailed",
      "formal": "Formal",
      "minimal": "Minimal",
      "simple": "Simple",
      "snakeCase": "snake_case",
      "spacious": "Spacious",
      "structured": "Structured"
    },
    "noPatterns": "No style patterns",
    "noTemplates": "No style templates",
    "patterns": "Patterns",
    "presets": {
      "maximal": "Maximal",
      "minimal": "Minimal",
      "neutral": "Neutral"
    },
    "showMorePatterns": "Show more patterns",
    "showMoreTemplates": "Show more templates",
    "sourceStyle": "Source Style",
    "targetStyle": "Target Style",
    "templates": "Templates",
    "usedCount": "Used {{count}} times",
    "variants": "Variants"
  },
  "wiki": {
    "common": {
      "actions": "Actions",
      "back": "Back",
      "close": "Close",
      "overview": "Overview",
      "refresh": "Refresh"
    },
    "contentPlaceholder": "Enter content...",
    "emptyNotes": "No notes yet",
    "graph": {
      "edges": "Edges",
      "empty": "No graph data",
      "filterByTags": "Filter by tags",
      "filterByType": "Filter by type",
      "filters": "Filters",
      "loadError": "Load failed",
      "loading": "Loading...",
      "noWikiId": "No Wiki ID specified",
      "nodes": "Nodes",
      "stats": "Stats",
      "title": "Knowledge Graph",
      "type": {
        "concept": "Concept",
        "entity": "Entity",
        "note": "Note",
        "source": "Source"
      }
    },
    "ingest": {
      "docx": "Word Document",
      "file": "File",
      "folder": "Folder",
      "folderPath": "Folder path",
      "folderPathPlaceholder": "Enter folder path...",
      "folderPathRequired": "Folder path is required",
      "history": "Import history",
      "markdown": "Markdown",
      "notion": "Notion",
      "path": "Path",
      "pathPlaceholder": "Enter path...",
      "pdf": "PDF",
      "results": "Import results",
      "sourceType": "Source type",
      "sourceTypeRequired": "Source type is required",
      "start": "Start import",
      "title": "Import Source",
      "titlePlaceholder": "Enter title...",
      "type": "Type",
      "upload": "Upload",
      "uploadFile": "Upload file",
      "uploadHint": "Drop files or click to upload",
      "url": "URL",
      "urlInvalid": "Invalid URL format",
      "urlRequired": "URL is required",
      "web": "Web"
    },
    "insertLink": "Insert Link",
    "lastCompiled": "Last compiled",
    "lastLinted": "Last linted",
    "lint": {
      "averageScore": "Average score",
      "code": "Code",
      "details": "Details",
      "issues": "Issues",
      "line": "Line",
      "loadError": "Load failed",
      "message": "Message",
      "noIssues": "No issues",
      "noResults": "No results",
      "note": "Note",
      "rerun": "Re-run",
      "runLint": "Run lint",
      "score": "Score",
      "scoreUpdated": "Score updated",
      "selectNote": "Select note",
      "severity": "Severity",
      "success": "Lint complete",
      "totalIssues": "Total issues",
      "totalNotes": "Total notes",
      "updateScore": "Update score",
      "viewDetails": "View details"
    },
    "llm": {
      "backToList": "Back to list",
      "compile": "Compile",
      "compileErrors": "Compile errors",
      "compileSuccess": "Compiled successfully",
      "confirmDelete": "Confirm delete?",
      "create": "Create",
      "createSuccess": "Created successfully",
      "createWiki": "Create Wiki",
      "deleteSuccess": "Deleted successfully",
      "descriptionPlaceholder": "Enter description...",
      "fileUploaded": "File uploaded",
      "ingestError": "Import failed",
      "ingestSource": "Import source",
      "ingestSuccess": "Imported successfully",
      "lintReport": "Lint report",
      "lowQualityWarning": "Low quality, improvements recommended",
      "namePlaceholder": "Enter name...",
      "nameRequired": "Name is required",
      "newCompiledContent": "New compiled content",
      "noScore": "No score yet",
      "operations": "Operations",
      "pathPlaceholder": "Enter path...",
      "pathRequired": "Path is required",
      "readyToSave": "Ready to save",
      "recompile": "Recompile",
      "select": "Select",
      "selectSourcesFirst": "Please select sources first",
      "sources": "Sources",
      "stats": {
        "lastCompile": "Last compile",
        "operations": "Operations",
        "sources": "Sources",
        "totalWikis": "Total Wikis"
      },
      "title": "Wiki Management",
      "uploadError": "Upload failed",
      "viewInGraph": "View in graph",
      "viewSource": "View source",
      "wikiList": "Wiki List"
    },
    "llmNote": "LLM Note",
    "log": {
      "completed": "Completed",
      "failed": "Failed",
      "filterStatus": "Filter by status",
      "filterType": "Filter by type",
      "noOperations": "No operations",
      "operationDetail": "Operation detail",
      "running": "Running",
      "title": "Operation Log",
      "total": "Total"
    },
    "noSources": "No sources",
    "noteNotFound": "Note not found",
    "notes": "Notes",
    "operation": {
      "compile": "Compile",
      "completedAt": "Completed at",
      "createdAt": "Created at",
      "details": "Details",
      "duration": "Duration",
      "error": "Error",
      "id": "ID",
      "ingest": "Ingest",
      "lint": "Lint",
      "noOperations": "No operations",
      "page": "Page",
      "result": "Result",
      "schema": "Schema",
      "source": "Source",
      "status": "Status",
      "sync": "Sync",
      "type": "Type"
    },
    "pageNotFound": "Page not found",
    "pageType": "Page Type",
    "quality": {
      "excellent": "Excellent",
      "factors": "Quality Factors",
      "fair": "Fair",
      "good": "Good",
      "issueList": "Issue List",
      "issues": "Issues",
      "more": "More",
      "noData": "No data",
      "poor": "Poor",
      "refresh": "Refresh",
      "title": "Quality Assessment"
    },
    "qualityScore": "Quality Score",
    "save": "Save",
    "saved": "Saved",
    "schema": {
      "create": "Create",
      "created": "Created",
      "createdAt": "Created at",
      "deleteConfirm": "Confirm delete?",
      "deleted": "Deleted",
      "description": "Description",
      "descriptionPlaceholder": "Enter description...",
      "edit": "Edit",
      "noSchemas": "No schemas",
      "schema": "Schema",
      "schemaHelp": "Define the data structure of the Wiki",
      "title": "Schema Management",
      "updated": "Updated",
      "version": "Version"
    },
    "searchPlaceholder": "Search Wiki...",
    "selectWiki": "Select Wiki",
    "selectWikiPrompt": "Please select a Wiki",
    "source": {
      "chunks": "Chunks",
      "deleteNotImplemented": "Delete not implemented",
      "path": "Path",
      "status": "Status",
      "title": "Source",
      "type": "Type"
    },
    "sources": "Sources",
    "sync": {
      "byWiki": "By Wiki",
      "capacity": "Capacity",
      "emptyQueue": "Queue is empty",
      "failed": "Sync failed",
      "linkCreated": "Link created",
      "linkDeleted": "Link deleted",
      "more": "More",
      "noteCreated": "Note created",
      "noteDeleted": "Note deleted",
      "noteUpdated": "Note updated",
      "pending": "Pending",
      "processNow": "Process now",
      "processStarted": "Processing started",
      "processing": "Processing...",
      "queue": "Queue",
      "refresh": "Refresh",
      "retryCount": "Retry count",
      "title": "Sync",
      "usage": "Usage"
    },
    "titlePlaceholder": "Enter title...",
    "userNote": "User Note",
    "wiki": {
      "description": "Description",
      "name": "Name",
      "rootPath": "Root Path",
      "schemaVersion": "Schema Version"
    }
  },
  "workEngine": {
    "cancel": "Cancel",
    "cancelExecution": "Cancel Execution",
    "confirmCancelExecution": "Confirm cancel current execution?",
    "duration": "Duration",
    "executionHistory": "Execution History",
    "executionId": "Execution ID",
    "pause": "Pause",
    "pauseExecution": "Pause Execution",
    "resume": "Resume",
    "resumeExecution": "Resume Execution",
    "start": "Start",
    "startExecution": "Start Execution",
    "startTime": "Start Time",
    "status": "Status",
    "statusCancelled": "Cancelled",
    "statusCompleted": "Completed",
    "statusFailed": "Failed",
    "statusPaused": "Paused",
    "statusPending": "Pending",
    "statusRunning": "Running"
  },
  "workflow": {
    "applySemanticCheck": "Apply",
    "applyUpgrade": "Apply Upgrade",
    "category": "Category",
    "decompositionSaved": "Decomposition saved",
    "description": "Description",
    "entryType": "Entry Type",
    "existingSkill": "Existing Skill",
    "generatedSkill": "Generated Skill",
    "generatingUpgrade": "Generating upgrade suggestion...",
    "inputSchema": "Input Schema",
    "keepGeneratedSkill": "Keep Generated",
    "matchReasons": "Match Reasons",
    "outputSchema": "Output Schema",
    "replaceExisting": "Replace Existing",
    "replaceWithExisting": "Use Existing",
    "saveAsNew": "Save as New",
    "saved": "Saved",
    "semanticCheckApplied": "Semantic check applied",
    "semanticCheckDescription": "The following generated skills have high semantic similarity with existing skills",
    "semanticCheckTitle": "Similar Skills Found",
    "similarWorkflowsExplanation": "Found workflows similar to the current one",
    "similarWorkflowsFound": "Similar workflows found",
    "similarity": "Similarity",
    "skills": "Skills",
    "skipSemanticCheck": "Skip",
    "upgradeError": "Upgrade failed",
    "upgradeExisting": "Upgrade Existing",
    "upgradeReasoning": "Upgrade Reasoning",
    "upgradeSkillTitle": "Upgrade Skill",
    "upgradeSuggestion": "Upgrade Suggestion",
    "upgradedName": "Upgraded Name",
    "validationFailed": "Validation failed",
    "workflowSavedAsNew": "Workflow saved as new",
    "workflowUpdated": "Workflow updated"
  }
}


def deep_merge(base, additions):
    """Deep merge additions into base dict, adding only missing keys."""
    for key, value in additions.items():
        if key in base:
            if isinstance(base[key], dict) and isinstance(value, dict):
                deep_merge(base[key], value)
        else:
            base[key] = value
    return base


def flatten_keys(d, prefix=''):
    """Get all flat key paths from a nested dict."""
    keys = set()
    for k, v in d.items():
        full_key = f'{prefix}.{k}' if prefix else k
        if isinstance(v, dict):
            keys.update(flatten_keys(v, full_key))
        else:
            keys.add(full_key)
    return keys


def main():
    # Step 1: Add missing keys to zh-CN
    zh_path = os.path.join(LOCALES_DIR, 'zh-CN.json')
    with open(zh_path, 'r', encoding='utf-8') as f:
        zh_data = json.load(f)

    zh_before = len(flatten_keys(zh_data))
    deep_merge(zh_data, ZH_MISSING)
    zh_after = len(flatten_keys(zh_data))

    with open(zh_path, 'w', encoding='utf-8') as f:
        json.dump(zh_data, f, ensure_ascii=False, indent=2)

    print(f'zh-CN.json: {zh_before} → {zh_after} keys (+{zh_after - zh_before})')

    # Step 2: Add missing keys to en-US
    en_path = os.path.join(LOCALES_DIR, 'en-US.json')
    with open(en_path, 'r', encoding='utf-8') as f:
        en_data = json.load(f)

    en_before = len(flatten_keys(en_data))
    deep_merge(en_data, EN_MISSING)
    en_after = len(flatten_keys(en_data))

    with open(en_path, 'w', encoding='utf-8') as f:
        json.dump(en_data, f, ensure_ascii=False, indent=2)

    print(f'en-US.json: {en_before} → {en_after} keys (+{en_after - en_before})')

    # Step 3: For other locales, add all missing keys using en-US as fallback
    # First build the complete en-US reference
    complete_en = en_data  # now has all keys

    other_locales = ['ar', 'de', 'es', 'fr', 'hi', 'ja', 'ko', 'ru', 'zh-TW']
    for locale in other_locales:
        fname = f'{locale}.json'
        fpath = os.path.join(LOCALES_DIR, fname)

        with open(fpath, 'r', encoding='utf-8') as f:
            locale_data = json.load(f)

        before = len(flatten_keys(locale_data))

        # Deep merge all missing keys from en-US as fallback
        deep_merge(locale_data, complete_en)

        after = len(flatten_keys(locale_data))

        with open(fpath, 'w', encoding='utf-8') as f:
            json.dump(locale_data, f, ensure_ascii=False, indent=2)

        added = after - before
        print(f'{fname}: {before} → {after} keys (+{added})')


if __name__ == '__main__':
    main()
