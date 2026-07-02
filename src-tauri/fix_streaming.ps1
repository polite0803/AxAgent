[byte[]]$bytes = [System.IO.File]::ReadAllBytes('D:\OneManager\AxAgent\src-tauri\src\commands\conversations\streaming.rs')
$pattern = [byte[]]@(0xEF, 0xBF, 0xBD, 0x3F)
$replacement = [byte[]]@(0x3F)  # ASCII '?'

# Build new content
$newBytes = New-Object System.Collections.Generic.List[byte]
$i = 0
$count = 0
while ($i -lt $bytes.Length) {
    if ($i -le $bytes.Length - 4 -and
        $bytes[$i] -eq 0xEF -and
        $bytes[$i+1] -eq 0xBF -and
        $bytes[$i+2] -eq 0xBD -and
        $bytes[$i+3] -eq 0x3F) {
        # Replace the 4-byte pattern with 1-byte '?'
        $newBytes.Add(0x3F) | Out-Null
        $i += 4
        $count++
    } else {
        $newBytes.Add($bytes[$i]) | Out-Null
        $i++
    }
}
Write-Host "Replaced $count occurrences of U+FFFD+? pattern"

# Write back to file using .NET
$path = 'D:\OneManager\AxAgent\src-tauri\src\commands\conversations\streaming.rs'
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)
[System.IO.File]::WriteAllBytes($path, $newBytes.ToArray())
Write-Host "File written"
