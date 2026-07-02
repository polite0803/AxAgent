[byte[]]$bytes = [System.IO.File]::ReadAllBytes('D:\OneManager\AxAgent\src-tauri\src\commands\conversations\streaming.rs')
# Around position 1107 (line 1107 in file). Need to find byte offset
# Read raw UTF8 around byte 44000-46000
$start = 44000
$end = 47000
for ($i = $start; $i -lt $end - 3; $i++) {
    if ($bytes[$i] -eq 0x0A) {
        # Find start of line
        $lineStart = $i + 1
        $lineEnd = $i
        while ($lineEnd -lt $end - 1 -and $bytes[$lineEnd + 1] -ne 0x0A) { $lineEnd++ }
        $line = ''
        for ($j = $lineStart; $j -le $lineEnd; $j++) {
            if ($bytes[$j] -eq 0xEF -and $j + 2 -lt $bytes.Length -and $bytes[$j+1] -eq 0xBF -and $bytes[$j+2] -eq 0xBD) {
                $line += '[U+FFFD]'
                $j += 2
            } elseif ($bytes[$j] -ge 0x20 -and $bytes[$j] -lt 0x7F) {
                $line += [char]$bytes[$j]
            } elseif ($bytes[$j] -lt 0x80) {
                $line += ('[0x{0:X2}]' -f $bytes[$j])
            } else {
                $line += ('[U+{0:X4}]' -f ([System.Text.Encoding]::UTF8.GetString($bytes, $j, [Math]::Min(3, $bytes.Length - $j))[0]))
            }
        }
        Write-Host $line
    }
}
