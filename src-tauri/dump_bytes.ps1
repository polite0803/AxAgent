[byte[]]$bytes = [System.IO.File]::ReadAllBytes('D:\OneManager\AxAgent\src-tauri\src\commands\conversations\streaming.rs')
# Look at bytes around 45500 to 45700 (where the first replacement char is)
$start = 45400
$end = 45750
$line = ''
$lineStart = -1
for ($i = $start; $i -lt $end; $i++) {
    if ($lineStart -eq -1 -and $bytes[$i] -eq 0x0A) { $lineStart = $i + 1 }
    if ($bytes[$i] -eq 0x0A) {
        Write-Host ("---line at $lineStart ---")
        for ($j = $lineStart; $j -le $i; $j++) {
            $c = [char]$bytes[$j]
            if ($bytes[$j] -ge 0x20 -and $bytes[$j] -lt 0x7F) {
                Write-Host -NoNewline $c
            } else {
                Write-Host -NoNewline ('[0x{0:X2}]' -f $bytes[$j])
            }
        }
        Write-Host ''
        $lineStart = $i + 1
    }
}
