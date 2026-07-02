[byte[]]$bytes = [System.IO.File]::ReadAllBytes('D:\OneManager\AxAgent\src-tauri\src\commands\memory.rs')
# Find line 126 byte range
$line = 1
$lineStart = 0
$lineEnd = 0
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
Write-Host "Line 126 bytes: $lineStart to $lineEnd"
# Show the bytes
for ($i = $lineStart; $i -le $lineEnd; $i++) {
    Write-Host ("  byte $i : 0x{0:X2}" -f $bytes[$i])
}

# Find the broken em-dash pattern: E2 80 3F 6D
$brokenPattern = [byte[]]@(0xE2, 0x80, 0x3F, 0x6D)
$replacement = [byte[]]@(0xE2, 0x80, 0x94, 0x20)  # em-dash + space

# Build new content
$newBytes = New-Object System.Collections.Generic.List[byte]
$i = 0
$count = 0
while ($i -lt $bytes.Length) {
    if ($i -le $bytes.Length - 4 -and
        $bytes[$i] -eq 0xE2 -and
        $bytes[$i+1] -eq 0x80 -and
        $bytes[$i+2] -eq 0x3F -and
        $bytes[$i+3] -eq 0x6D) {
        $newBytes.AddRange($replacement) | Out-Null
        $i += 4
        $count++
    } else {
        $newBytes.Add($bytes[$i]) | Out-Null
        $i++
    }
}
Write-Host "Replaced $count occurrences of broken em-dash pattern"

# Write back
$path = 'D:\OneManager\AxAgent\src-tauri\src\commands\memory.rs'
[System.IO.File]::WriteAllBytes($path, $newBytes.ToArray())
Write-Host "File written"
