[byte[]]$bytes = [System.IO.File]::ReadAllBytes('D:\OneManager\AxAgent\src-tauri\src\commands\conversations\streaming.rs')
# Find line 1107 - byte range 45272 to 45362
$lineContent = ''
$raw = ''
for ($i = 45272; $i -le 45362; $i++) {
    $hex = '0x{0:X2}' -f $bytes[$i]
    $raw += $hex + ' '
    if ($bytes[$i] -ge 0x20 -and $bytes[$i] -lt 0x7F) {
        $lineContent += [char]$bytes[$i]
    } elseif ($bytes[$i] -lt 0x80) {
        $lineContent += ('[{0:X2}]' -f $bytes[$i])
    } else {
        $lineContent += ('[U+{0:X4}]' -f ([System.Text.Encoding]::UTF8.GetString($bytes, $i, 3))[0])
        $i += 2
        $raw += '... '
    }
}
Write-Host "Line 1107 content: $lineContent"
Write-Host "Raw bytes: $raw"
