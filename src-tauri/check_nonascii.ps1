[byte[]]$bytes = [System.IO.File]::ReadAllBytes('D:\OneManager\AxAgent\src-tauri\src\commands\memory.rs')
Write-Host "File size: $($bytes.Length) bytes"
# Check for any non-ASCII bytes
$nonAscii = @()
for ($i = 0; $i -lt $bytes.Length; $i++) {
    if ($bytes[$i] -ge 0x80) {
        $nonAscii += $i
    }
}
Write-Host "Non-ASCII byte count: $($nonAscii.Count)"
# Show first 30
Write-Host "First 30 non-ASCII positions:"
$idx = 0
foreach ($pos in $nonAscii[0..29]) {
    $start = [Math]::Max(0, $pos - 5)
    $end = [Math]::Min($bytes.Length - 1, $pos + 5)
    Write-Host ("Position {0}: 0x{1:X2} context:" -f $pos, $bytes[$pos])
    for ($i = $start; $i -le $end; $i++) {
        if ($bytes[$i] -ge 0x20 -and $bytes[$i] -lt 0x7F) {
            Write-Host -NoNewline ([char]$bytes[$i])
        } else {
            Write-Host -NoNewline ('[0x{0:X2}]' -f $bytes[$i])
        }
    }
    Write-Host ''
    $idx++
    if ($idx -ge 30) { break }
}
