[byte[]]$bytes = [System.IO.File]::ReadAllBytes('D:\OneManager\AxAgent\src-tauri\src\commands\conversations\streaming.rs')
# Look at byte 45417 to 45425
$start = 45417
$end = 45425
for ($i = $start; $i -le $end; $i++) {
    $hex = '0x{0:X2}' -f $bytes[$i]
    $c = if ($bytes[$i] -ge 0x20 -and $bytes[$i] -lt 0x7F) { [char]$bytes[$i] } else { '.' }
    Write-Host "byte $i : $hex ($c)"
}
Write-Host '---'
# Look at byte 45420-45430 in more detail (UTF-8 boundary aware)
$pos = 45420
$context = ''
for ($i = 45410; $i -lt 45430; $i++) {
    if ($bytes[$i] -ge 0x20 -and $bytes[$i] -lt 0x7F) {
        $context += [char]$bytes[$i]
    } else {
        $context += ('[0x{0:X2}]' -f $bytes[$i])
    }
}
Write-Host "Context: $context"
