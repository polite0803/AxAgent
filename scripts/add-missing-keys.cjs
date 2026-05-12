const fs = require("fs");

const missingKeys = [
  // chatMarkdown
  "chatMarkdown.toggleRaw",
  "chatMarkdown.viewThinking",
  "chatMarkdown.inputParams",
  "chatMarkdown.outputResult",

  // chat.context
  "chat.context.tokenMax",

  // chat.diff
  "chat.diff.accept",
  "chat.diff.reject",
  "chat.diff.accepted",
  "chat.diff.rejected",
  "chat.diff.fileChanges",
  "chat.diff.collapseAll",
  "chat.diff.expandAll",
  "chat.diff.acceptAll",

  // chat
  "chat.selfEvolution",
  "chat.tracer",
  "chat.tracerTitle",
  "chat.traces",
  "chat.spans",
  "chat.errors",
  "chat.active",
  "chat.noTraces",

  // plan.status
  "plan.status.reviewing",
  "plan.status.executing",
  "plan.justNow",
  "plan.minutesAgo",
  "plan.hoursAgo",

  // error
  "error.page",

  // advanced
  "advanced.bashSecurity",
  "advanced.dangerousCmdDetect",
  "advanced.networkCmdDetect",
  "advanced.cmdTimeout",
  "advanced.permissionStrategy",
  "advanced.defaultPermission",
  "advanced.perm.default",
  "advanced.perm.acceptEdits",
  "advanced.perm.fullAccess",
  "advanced.fileWriteConfirm",
  "advanced.networkConfirm",
  "advanced.shellConfirm",

  // common
  "common.operationFailed",
  "common.hide",
  "common.searchPlaceholder",

  // hookLog
  "hookLog.column.time",
  "hookLog.column.event",
  "hookLog.column.tool",
  "hookLog.column.command",
  "hookLog.column.status",
  "hookLog.success",
  "hookLog.failed",
  "hookLog.column.outputSummary",
  "hookLog.filterLabel",
  "hookLog.allEvents",
  "hookLog.recordCount",
  "hookLog.clear",
  "hookLog.emptyStatus",
  "hookLog.emptyFiltered",

  // mcp
  "mcp.enabled",
  "mcp.disabled",

  // settings.memory
  "settings.memory.promoteSuccess",
  "settings.memory.demoteSuccess",
  "settings.memory.feedbackSuccess",
  "settings.memory.ageJustNow",
  "settings.memory.ageMinutes",
  "settings.memory.ageHours",
  "settings.memory.ageDays",
  "settings.memory.noWorkingMemory",
  "settings.memory.clusterLabel",
  "settings.memory.avgImportance",
  "settings.memory.noKnowledgeGraph",
  "settings.memory.target",

  // settings
  "settings.expertPromptNote",
  "settings.modelGroupPlaceholder",
  "settings.localTools.empty",
  "settings.workflow.myWorkflows",
  "settings.workflow.marketplace",

  // settings.scheduler
  "settings.scheduler.executed",
  "settings.scheduler.quickCreated",
  "settings.scheduler.refreshed",

  // scheduler
  "scheduler.refresh",
  "scheduler.dailyReport",
  "scheduler.backup",
  "scheduler.cleanup",
  "scheduler.executeNow",
  "scheduler.executionHistory",
  "scheduler.historyTitle",
  "scheduler.noRecords",
  "scheduler.success",
  "scheduler.failed",

  // profile
  "profile.description",
  "profile.codingStyleDesc",
  "profile.namingConventionDesc",
  "profile.indentationStyleDesc",
  "profile.commentStyleDesc",
  "profile.communicationDesc",
  "profile.detailLevelDesc",
  "profile.toneDesc",
  "profile.languageDesc",
  "profile.confidenceDesc",

  // wiki
  "wiki.backlinks",
  "wiki.noBacklinks",
  "wiki.tags",
  "wiki.versionRestored",
  "wiki.versionHistory",
  "wiki.noVersions",
  "wiki.compareDiff",
  "wiki.confirmRestore",
  "wiki.restore",
  "wiki.diffLabel",
  "wiki.selectNote",
  "wiki.close",
  "wiki.edit",
  "wiki.unsaved",
  "wiki.outlinks",
  "wiki.noOutlinks",
  "wiki.localGraph",
  "wiki.noTags",
  "wiki.viewBacklinks",
  "wiki.focusLocal",
  "wiki.createLinkedNote",
  "wiki.delete",
  "wiki.cancel",

  // top-level (markdown, success, error)
  "markdown",
  "success",
  "error",
];

// English translations
const enTranslations = {
  "chatMarkdown.toggleRaw": "Toggle Raw",
  "chatMarkdown.viewThinking": "View Thinking",
  "chatMarkdown.inputParams": "Input Params",
  "chatMarkdown.outputResult": "Output Result",
  "chat.context.tokenMax": "Token Limit",
  "chat.diff.accept": "Accept",
  "chat.diff.reject": "Reject",
  "chat.diff.accepted": "Accepted",
  "chat.diff.rejected": "Rejected",
  "chat.diff.fileChanges": "File Changes",
  "chat.diff.collapseAll": "Collapse All",
  "chat.diff.expandAll": "Expand All",
  "chat.diff.acceptAll": "Accept All",
  "chat.selfEvolution": "Self Evolution",
  "chat.tracer": "Tracer",
  "chat.tracerTitle": "Tracer Title",
  "chat.traces": "Traces",
  "chat.spans": "Spans",
  "chat.errors": "Errors",
  "chat.active": "Active",
  "chat.noTraces": "No traces",
  "plan.status.reviewing": "Reviewing",
  "plan.status.executing": "Executing",
  "plan.justNow": "Just now",
  "plan.minutesAgo": "{{count}} minutes ago",
  "plan.hoursAgo": "{{count}} hours ago",
  "error.page": "Error",
  "advanced.bashSecurity": "Bash Security",
  "advanced.dangerousCmdDetect": "Dangerous Command Detection",
  "advanced.networkCmdDetect": "Network Command Detection",
  "advanced.cmdTimeout": "Command Timeout",
  "advanced.permissionStrategy": "Permission Strategy",
  "advanced.defaultPermission": "Default Permission",
  "advanced.perm.default": "Default",
  "advanced.perm.acceptEdits": "Accept Edits",
  "advanced.perm.fullAccess": "Full Access",
  "advanced.fileWriteConfirm": "File Write Confirmation",
  "advanced.networkConfirm": "Network Confirmation",
  "advanced.shellConfirm": "Shell Confirmation",
  "common.operationFailed": "Operation failed",
  "common.hide": "Hide",
  "common.searchPlaceholder": "Search...",
  "hookLog.column.time": "Time",
  "hookLog.column.event": "Event",
  "hookLog.column.tool": "Tool",
  "hookLog.column.command": "Command",
  "hookLog.column.status": "Status",
  "hookLog.success": "Success",
  "hookLog.failed": "Failed",
  "hookLog.column.outputSummary": "Output Summary",
  "hookLog.filterLabel": "Filter",
  "hookLog.allEvents": "All Events",
  "hookLog.recordCount": "{{count}} records",
  "hookLog.clear": "Clear",
  "hookLog.emptyStatus": "No status",
  "hookLog.emptyFiltered": "No matching records",
  "mcp.enabled": "Enabled",
  "mcp.disabled": "Disabled",
  "settings.memory.promoteSuccess": "Memory promoted",
  "settings.memory.demoteSuccess": "Memory demoted",
  "settings.memory.feedbackSuccess": "Feedback recorded",
  "settings.memory.ageJustNow": "Just now",
  "settings.memory.ageMinutes": "{{count}} minutes",
  "settings.memory.ageHours": "{{count}} hours",
  "settings.memory.ageDays": "{{count}} days",
  "settings.memory.noWorkingMemory": "No working memory",
  "settings.memory.clusterLabel": "Cluster",
  "settings.memory.avgImportance": "Avg Importance",
  "settings.memory.noKnowledgeGraph": "No knowledge graph",
  "settings.memory.target": "Target",
  "settings.expertPromptNote": "Expert prompt note",
  "settings.modelGroupPlaceholder": "Select model group",
  "settings.localTools.empty": "No local tools",
  "settings.workflow.myWorkflows": "My Workflows",
  "settings.workflow.marketplace": "Marketplace",
  "settings.scheduler.executed": "Executed",
  "settings.scheduler.quickCreated": "Quick created",
  "settings.scheduler.refreshed": "Refreshed",
  "scheduler.refresh": "Refresh",
  "scheduler.dailyReport": "Daily Report",
  "scheduler.backup": "Backup",
  "scheduler.cleanup": "Cleanup",
  "scheduler.executeNow": "Execute Now",
  "scheduler.executionHistory": "Execution History",
  "scheduler.historyTitle": "History",
  "scheduler.noRecords": "No records",
  "scheduler.success": "Success",
  "scheduler.failed": "Failed",
  "profile.description": "Description",
  "profile.codingStyleDesc": "Coding Style",
  "profile.namingConventionDesc": "Naming Convention",
  "profile.indentationStyleDesc": "Indentation Style",
  "profile.commentStyleDesc": "Comment Style",
  "profile.communicationDesc": "Communication",
  "profile.detailLevelDesc": "Detail Level",
  "profile.toneDesc": "Tone",
  "profile.languageDesc": "Language",
  "profile.confidenceDesc": "Confidence",
  "wiki.backlinks": "Backlinks",
  "wiki.noBacklinks": "No backlinks",
  "wiki.tags": "Tags",
  "wiki.versionRestored": "Version restored",
  "wiki.versionHistory": "Version History",
  "wiki.noVersions": "No versions",
  "wiki.compareDiff": "Compare Diff",
  "wiki.confirmRestore": "Confirm restore?",
  "wiki.restore": "Restore",
  "wiki.diffLabel": "Diff",
  "wiki.selectNote": "Select note",
  "wiki.close": "Close",
  "wiki.edit": "Edit",
  "wiki.unsaved": "Unsaved",
  "wiki.outlinks": "Outlinks",
  "wiki.noOutlinks": "No outlinks",
  "wiki.localGraph": "Local Graph",
  "wiki.noTags": "No tags",
  "wiki.viewBacklinks": "View Backlinks",
  "wiki.focusLocal": "Focus Local",
  "wiki.createLinkedNote": "Create Linked Note",
  "wiki.delete": "Delete",
  "wiki.cancel": "Cancel",
  "markdown": "Markdown",
  "success": "Success",
  "error": "Error",
};

// Load all locale files
const locales = ["ar", "de", "es", "fr", "hi", "ja", "ko", "ru", "zh-TW", "zh-CN", "en-US"];
const localeData = {};

for (const loc of locales) {
  const path = `src/i18n/locales/${loc}.json`;
  localeData[loc] = JSON.parse(fs.readFileSync(path, "utf8"));
}

// Add keys to each locale
function addKey(obj, key, value) {
  const parts = key.split(".");
  let current = obj;
  for (let i = 0; i < parts.length - 1; i++) {
    const p = parts[i];
    if (!current[p]) { current[p] = {}; }
    current = current[p];
  }
  current[parts[parts.length - 1]] = value;
}

for (const loc of locales) {
  for (const key of missingKeys) {
    const enValue = enTranslations[key];
    if (enValue) {
      // For non-en-US locales, use English as placeholder if no translation exists
      const existingValue = localeData[loc];
      let exists = true;
      const parts = key.split(".");
      let current = existingValue;
      for (const p of parts) {
        if (!current || typeof current !== "object") {
          exists = false;
          break;
        }
        current = current[p];
      }
      if (!exists || current === undefined) {
        addKey(localeData[loc], key, loc === "en-US" ? enValue : enValue);
      }
    }
  }

  // Write back
  fs.writeFileSync(`src/i18n/locales/${loc}.json`, JSON.stringify(localeData[loc], null, 2) + "\n");
  console.log(`Updated ${loc}.json`);
}

console.log(`\nAdded ${missingKeys.length} missing keys to all locale files`);
