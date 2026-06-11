$files = Get-ChildItem -Recurse -Include *.rs -Path . | Where-Object { $_.FullName -notmatch 'tests' -and $_.FullName -notmatch 'target' -and $_.FullName -notmatch '\.bak' }
$total = 0
$perFile = @{}
foreach ($file in $files) {
    $content = Get-Content $file.FullName -Raw
    # Strip all test blocks (any `#[cfg(test)]` and subsequent `mod tests { ... }` blocks)
    # For simplicity, find the position of `#[cfg(test)]` and only count up to that point.
    $idx = $content.IndexOf('#[cfg(test)]')
    if ($idx -ge 0) {
        $prodContent = $content.Substring(0, $idx)
    } else {
        $prodContent = $content
    }
    $count = ([regex]::Matches($prodContent, '\.unwrap\(\)|\.expect\(')).Count
    if ($count -gt 0) {
        $perFile[$file.Name] = $count
        $total += $count
    }
}
Write-Output "Total production unwrap/expect: $total"
$perFile.GetEnumerator() | Sort-Object Value -Descending | Select-Object -First 25 | Format-Table -AutoSize
