[byte[]]$bytes = [System.IO.File]::ReadAllBytes('D:\OneManager\AxAgent\src-tauri\src\commands\conversations\streaming.rs')

# Find all replacement char positions
$positions = @()
for ($i = 0; $i -lt $bytes.Length - 3; $i++) {
    if ($bytes[$i] -eq 0xEF -and $bytes[$i+1] -eq 0xBF -and $bytes[$i+2] -eq 0xBD) {
        $positions += $i
    }
}

# Show all 94 positions with context
$idx = 0
foreach ($pos in $positions) {
    $start = [Math]::Max(0, $pos - 50)
    $end = [Math]::Min($bytes.Length - 1, $pos + 50)
    Write-Host "[$idx] Position $pos ==="
    for ($i = $start; $i -le $end; $i++) {
        if ($bytes[$i] -eq 0xEF -and $i + 2 -lt $bytes.Length -and $bytes[$i+1] -eq 0xBF -and $bytes[$i+2] -eq 0xBD) {
            Write-Host -NoNewline '[U+FFFD]'
            $i += 2
        } elseif ($bytes[$i] -ge 0x20 -and $bytes[$i] -lt 0x7F) {
            Write-Host -NoNewline ([char]$bytes[$i])
        } elseif ($bytes[$i] -lt 0x80) {
            Write-Host -NoNewline ('[0x{0:X2}]' -f $bytes[$i])
        } else {
            Write-Host -NoNewline ('[U+{0:X}]' -f ([System.Text.Encoding]::UTF8.GetString($bytes, $i, [Math]::Min(3, $bytes.Length - $i))[0]))
        }
    }
    Write-Host ''
    $idx++
}
