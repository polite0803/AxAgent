# Fix common eslint patterns - use with caution
$files = @(
  "src/components/skill/FrontendEditorModal.tsx",
  "src/components/trace/BottleneckAnalyzer.tsx",
  "src/components/wiki/BacklinkPanel.tsx",
  "src/components/wiki/LintReport.tsx",
  "src/components/wiki/VersionHistoryPanel.tsx",
  "src/components/wiki/WikiDetailPanel.tsx",
  "src/components/workflow/DebugPanel.tsx",
  "src/lib/dynamicUI/useDataSource.ts",
  "src/pages/QuickBarPage.tsx",
  "src/components/settings/ProviderDetail.tsx",
  "src/components/settings/AcpSettings.tsx"
)

foreach ($f in $files) {
  $path = Join-Path "D:\OneManager\AxAgent" $f
  if (!(Test-Path $path)) { Write-Host "SKIP: $path not found"; continue }
  $content = Get-Content $path -Raw
  
  $original = $content
  
  # Fix pattern: setXxx(() => value) in useEffect
  $content = $content -replace '(?<=useEffect\(\s*\{\s*\n\s*)set(\w+)\(\s*\(\s*\)\s*=>\s*(.+?)\s*\)\s*;', 'setTimeout(() => set$1($2), 0);'
  
  if ($content -ne $original) {
    Set-Content -Path $path -Value $content -NoNewline
    Write-Host "FIXED: $f"
  } else {
    Write-Host "NO CHANGE: $f"
  }
}
