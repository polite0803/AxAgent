[byte[]]$bytes = [System.IO.File]::ReadAllBytes('D:\OneManager\AxAgent\src-tauri\src\commands\conversations\streaming.rs')

# Find all replacement char positions
$positions = @()
for ($i = 0; $i -lt $bytes.Length - 3; $i++) {
    if ($bytes[$i] -eq 0xEF -and $bytes[$i+1] -eq 0xBF -and $bytes[$i+2] -eq 0xBD) {
        $positions += $i
    }
}

# For each replacement, look at the surrounding bytes to see the context
foreach ($pos in $positions[0..4]) {
    $start = [Math]::Max(0, $pos - 30)
    $end = [Math]::Min($bytes.Length - 1, $pos + 30)
    Write-Host "=== Position $pos ==="
    for ($i = $start; $i -le $end; $i++) {
        if ($bytes[$i] -ge 0x20 -and $bytes[$i] -lt 0x7F) {
            Write-Host -NoNewline ([char]$bytes[$i])
        } elseif ($bytes[$i] -eq 0xEF -and $i + 2 -lt $bytes.Length -and $bytes[$i+1] -eq 0xBF -and $bytes[$i+2] -eq 0xBD) {
            Write-Host -NoNewline '[REPL]'
            $i += 2
        } elseif ($bytes[$i] -lt 0x80) {
            Write-Host -NoNewline ('[0x{0:X2}]' -f $bytes[$i])
        } else {
            Write-Host -NoNewline ('[0x{0:X2}]' -f $bytes[$i])
        }
    }
    Write-Host ''
}
