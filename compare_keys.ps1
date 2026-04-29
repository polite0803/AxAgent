$en = Get-Content "d:\OneManager\AxAgent\src\i18n\locales\en-US.json" -Raw | ConvertFrom-Json
$zh = Get-Content "d:\OneManager\AxAgent\src\i18n\locales\zh-CN.json" -Raw | ConvertFrom-Json

function Get-Keys($obj, $prefix) {
    $obj.PSObject.Properties | ForEach-Object {
        if ($_.Value -is [PSCustomObject]) {
            Get-Keys $_.Value ($prefix + $_.Name + ".")
        } else {
            $prefix + $_.Name
        }
    }
}

$enKeys = Get-Keys $en ""
$zhKeys = Get-Keys $zh ""
$missing = $enKeys | Where-Object { $_ -notin $zhKeys }

Write-Host "Missing keys: $($missing.Count)"
$missing | ForEach-Object { Write-Host $_ }
