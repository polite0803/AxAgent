Set-Location "d:\OneManager\AxAgent\src-tauri"
$env:RUST_BACKTRACE = "0"
cargo test -p axagent-storage --lib --no-fail-fast -- --test-threads=1 2>&1
exit $LASTEXITCODE
