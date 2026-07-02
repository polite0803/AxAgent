[byte[]]$bytes = [System.IO.File]::ReadAllBytes('D:\OneManager\AxAgent\src-tauri\src\commands\conversations\streaming.rs')
$pattern = @(0xEF, 0xBF, 0xBD)
$positions = @()
for ($i = 0; $i -lt $bytes.Length - 3; $i++) {
    if ($bytes[$i] -eq 0xEF -and $bytes[$i+1] -eq 0xBF -and $bytes[$i+2] -eq 0xBD) {
        $positions += $i
    }
}
Write-Host "Found $($positions.Count) replacement characters at byte positions:"
Write-Host ($positions -join ', ')
