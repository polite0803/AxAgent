# AxAgent DTO 命名统一 —— 自动替换脚本（基于 typecheck 错误行）
# 仅替换"错误行"上的 snake_case 字段访问/字面量键，保留 invoke 命令参数（snake_case）
$ErrorActionPreference = 'Stop'
$root = 'd:\OneManager\AxAgent'
$errFile = Join-Path $root 'my_errors.txt'

# 解析错误文件 -> 按文件分组的行级替换映射
$byFile = @{}
$unparseable = New-Object System.Collections.Generic.List[string]

foreach ($line in [System.IO.File]::ReadAllLines($errFile, [System.Text.Encoding]::UTF8)) {
    if ($line -match '^(.+?)\((\d+),\d+\): error TS\d+:\s*(.*)$') {
        $file = $matches[1].Trim()
        $ln = [int]$matches[2]
        $msg = $matches[3]
        $old = $null
        $new = $null
        if ($msg -match "Property '([^']+)' does not exist on type '.*'\. Did you mean '([^']+)'\?") {
            $old = $matches[1]; $new = $matches[2]
        } elseif ($msg -match "Object literal may only specify known properties, but '([^']+)' does not exist in type '.*'\. Did you mean to write '([^']+)'\?") {
            $old = $matches[1]; $new = $matches[2]
        }
        if ($null -ne $old -and $old -ne $new) {
            if (-not $byFile.ContainsKey($file)) { $byFile[$file] = @{} }
            if (-not $byFile[$file].ContainsKey($ln)) { $byFile[$file][$ln] = New-Object System.Collections.Generic.List[object] }
            $byFile[$file][$ln].Add([pscustomobject]@{ old = $old; new = $new })
        } else {
            $unparseable.Add($line)
        }
    } else {
        $unparseable.Add($line)
    }
}

Write-Host "PARSEABLE_FILES=$($byFile.Count) UNPARSEABLE=$($unparseable.Count)"

# 输出不可解析的错误行，供手工处理
$unparseable | Set-Content -Path (Join-Path $root 'manual_errors.txt') -Encoding UTF8

# 应用替换
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)
$changedFiles = 0
$totalRepl = 0
foreach ($file in ($byFile.Keys | Sort-Object)) {
    $path = Join-Path $root $file
    if (-not (Test-Path $path)) { Write-Host "MISSING: $path"; continue }
    $raw = [System.IO.File]::ReadAllText($path, [System.Text.Encoding]::UTF8)
    $lines = $raw.Split([char]10)
    $fileRepl = 0
    foreach ($ln in ($byFile[$file].Keys | Sort-Object)) {
        if ($ln -gt $lines.Count) { continue }
        $content = $lines[$ln - 1]
        $before = $content
        foreach ($r in $byFile[$file][$ln]) {
            $content = [regex]::Replace($content, ("\b" + [regex]::Escape($r.old) + "\b"), $r.new)
        }
        if ($content -ne $before) { $lines[$ln - 1] = $content; $fileRepl++ }
    }
    if ($fileRepl -gt 0) {
        $newRaw = $lines -join [char]10
        [System.IO.File]::WriteAllText($path, $newRaw, $utf8NoBom)
        $changedFiles++
        $totalRepl += $fileRepl
    }
}
Write-Host "CHANGED_FILES=$changedFiles TOTAL_EDITED_LINES=$totalRepl"
