$ErrorActionPreference = "Stop"
$root = "D:\OneManager\AxAgent"

function Fix-File($relPath, $fixes) {
  $path = Join-Path $root $relPath
  $content = Get-Content $path -Raw
  $orig = $content
  foreach ($fix in $fixes) {
    $content = $content -replace $fix[0], $fix[1]
  }
  if ($content -ne $orig) {
    Set-Content -Path $path -Value $content -NoNewline
    Write-Host "FIXED: $relPath"
  } else {
    Write-Host "SKIP: $relPath"
  }
}

# Fix set-state-in-effect patterns
Fix-File "src\components\wiki\BacklinkPanel.tsx" @(
  @('loadBacklinks();', 'setTimeout(() => loadBacklinks(), 0);')
)
Fix-File "src\components\wiki\LintReport.tsx" @(
  @('loadLintResults();', 'setTimeout(() => loadLintResults(), 0);')
)
Fix-File "src\components\wiki\VersionHistoryPanel.tsx" @(
  @('loadVersionList();', 'setTimeout(() => loadVersionList(), 0);')
)
Fix-File "src\components\workflow\DebugPanel.tsx" @(
  @('setSubDiags(() => ({}));', 'setTimeout(() => setSubDiags({}), 0);')
)
Fix-File "src\lib\dynamicUI\useDataSource.ts" @(
  @('setLoading(false);', 'setTimeout(() => setLoading(false), 0);'),
  @('setError(null);', 'setTimeout(() => setError(null), 0);')
)
Fix-File "src\pages\QuickBarPage.tsx" @(
  @('setSelectedCmd(0);', 'setTimeout(() => setSelectedCmd(0), 0);'),
  @('setShowCommands(false);', 'setTimeout(() => setShowCommands(false), 0);')
)

Write-Host "Done"
