[byte[]]$bytes = [System.IO.File]::ReadAllBytes('D:\OneManager\AxAgent\src-tauri\src\commands\memory.rs')
# Find line 126 - find newlines and count
$line = 1
$lineStart = 0
for ($i = 0; $i -lt $bytes.Length; $i++) {
    if ($bytes[$i] -eq 0x0A) {
        if ($line -eq 125) {
            $lineStart = $i + 1
        }
        if ($line -eq 126) {
            $lineEnd = $i - 1
            break
        }
        $line++
    }
}
Write-Host "Line 126 starts at byte $lineStart, ends at $lineEnd"
Write-Host "Line 126 content:"
for ($i = $lineStart; $i -le $lineEnd; $i++) {
    if ($bytes[$i] -ge 0x20 -and $bytes[$i] -lt 0x7F) {
        Write-Host -NoNewline ([char]$bytes[$i])
    } elseif ($bytes[$i] -lt 0x80) {
        Write-Host -NoNewline ('[0x{0:X2}]' -f $bytes[$i])
    } else {
        $hex = ''
        $len = [Math]::Min(4, $lineEnd - $i + 1)
        for ($k = 0; $k -lt $len; $k++) {
            $hex += '0x{0:X2} ' -f $bytes[$i + $k]
        }
        Write-Host -NoNewline ('[{0}]' -f $hex.TrimEnd())
    }
}
Write-Host ''
