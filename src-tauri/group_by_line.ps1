[byte[]]$bytes = [System.IO.File]::ReadAllBytes('D:\OneManager\AxAgent\src-tauri\src\commands\conversations\streaming.rs')
# Find all replacement char positions
$positions = @()
for ($i = 0; $i -lt $bytes.Length - 3; $i++) {
    if ($bytes[$i] -eq 0xEF -and $bytes[$i+1] -eq 0xBF -and $bytes[$i+2] -eq 0xBD) {
        $positions += $i
    }
}
# For each position, compute line number
$line = 1
$lineStart = 0
$lineMap = @{}  # pos -> lineNum
for ($i = 0; $i -lt $bytes.Length; $i++) {
    if ($bytes[$i] -eq 0x0A) {
        for ($j = $lineStart; $j -le $i; $j++) {
            $lineMap[$j] = $line
        }
        $lineStart = $i + 1
        $line++
    }
}

# Group positions by line and print context
$grouped = $positions | Group-Object { $lineMap[$_] }
foreach ($g in $grouped) {
    $lineNum = $g.Name
    Write-Host "=== Line $lineNum ==="
    foreach ($pos in $g.Group) {
        $start = [Math]::Max(0, $pos - 5)
        $end = [Math]::Min($bytes.Length - 1, $pos + 8)
        $ctx = ''
        for ($i = $start; $i -le $end; $i++) {
            if ($bytes[$i] -eq 0xEF -and $i + 2 -lt $bytes.Length -and $bytes[$i+1] -eq 0xBF -and $bytes[$i+2] -eq 0xBD) {
                $ctx += '<REPL>'
                $i += 2
            } elseif ($bytes[$i] -ge 0x20 -and $bytes[$i] -lt 0x7F) {
                $ctx += [char]$bytes[$i]
            } elseif ($bytes[$i] -lt 0x80) {
                $ctx += ('[{0:X2}]' -f $bytes[$i])
            } else {
                $ctx += ('[U+{0:X4}]' -f ([System.Text.Encoding]::UTF8.GetString($bytes, $i, [Math]::Min(3, $bytes.Length - $i))[0]))
            }
        }
        Write-Host "  byte $pos : ...$ctx..."
    }
}
