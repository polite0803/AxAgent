// Deleted - the trait approach for kit modules was too complex.
// Kit modules that agent uses internally (MarkdownParser, HtmlCleaner,
// TokenBudgetTracker, skill_dirs, slash_command) are kept as direct
// dependencies. Only compile_plan_to_dag re-export was removed.
