Set-Location "d:\OneManager\AxAgent\src-tauri"
$env:RUST_BACKTRACE = "0"
cargo test -p axagent-storage --lib storage_migration::tests::migrates_image_file --no-fail-fast 2>&1
exit $LASTEXITCODE
