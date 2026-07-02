[byte[]]$bytes = [System.IO.File]::ReadAllBytes('D:\OneManager\AxAgent\src-tauri\src\commands\conversations\streaming.rs')
$start = 0
for ($i = 50000; $i -lt 50100; $i++) {
    Write-Host ('{0,6}: 0x{1:X2} {2}' -f $i, $bytes[$i], [char]$bytes[$i])
}
