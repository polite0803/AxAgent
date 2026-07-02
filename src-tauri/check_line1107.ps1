[byte[]]$bytes = [System.IO.File]::ReadAllBytes('D:\OneManager\AxAgent\src-tauri\src\commands\conversations\streaming.rs')
# Find line 1107 byte range
$line = 1
$lineStart = 0
$lineEnd = 0
for ($i = 0; $i -lt $bytes.Length; $i++) {
    if ($bytes[$i] -eq 0x0A) {
        if ($line -eq 1106) {
            $lineStart = $i + 1
        }
        if ($line -eq 1107) {
            $lineEnd = $i - 1
            break
        }
        $line++
    }
}
Write-Host "Line 1107 bytes: $lineStart to $lineEnd"
# Show the bytes
$lineContent = ''
for ($i = $lineStart; $i -le $lineEnd; $i++) {
    if ($bytes[$i] -ge 0x20 -and $bytes[$i] -lt 0x7F) {
        $lineContent += [char]$bytes[$i]
    } elseif ($bytes[$i] -lt 0x80) {
        $lineContent += ('[{0:X2}]' -f $bytes[$i])
    } else {
        # 3-byte UTF-8 char
        $lineContent += ('[U+{0:X4}]' -f ([System.Text.Encoding]::UTF8.GetString($bytes, $i, 3))[0])
        $i += 2
    }
}
Write-Host "Line 1107: $lineContent"
