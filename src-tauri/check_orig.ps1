[byte[]]$orig = [System.IO.File]::ReadAllBytes('D:\temp\original_streaming.rs')
# Find all U+FFFD positions
$positions = @()
for ($i = 0; $i -lt $orig.Length - 3; $i++) {
    if ($orig[$i] -eq 0xEF -and $orig[$i+1] -eq 0xBF -and $orig[$i+2] -eq 0xBD) {
        $positions += $i
    }
}
Write-Host "Found $($positions.Count) U+FFFD chars in original"
# For each, look at the next 4 bytes
foreach ($pos in $positions[0..10]) {
    $bytes4 = ''
    for ($j = $pos; $j -lt [Math]::Min($orig.Length, $pos + 8); $j++) {
        $bytes4 += '0x{0:X2} ' -f $orig[$j]
    }
    Write-Host "Pos $pos : $bytes4"
}
