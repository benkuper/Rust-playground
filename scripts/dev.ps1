$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$workspace = Split-Path -Parent $root

$env:GOLDEN_PORT = "9010"
$env:VITE_GOLDEN_SERVER = "http://localhost:9010"

Write-Host "Starting SvelteKit dev server..."
$ui = Start-Process -FilePath "npm" -ArgumentList "--prefix", "src-ui", "run", "dev" -WorkingDirectory $workspace -PassThru

Write-Host "Starting Tauri app (auto-rebuild on Rust changes)..."
$tauri = Start-Process -FilePath "cargo" -ArgumentList "watch", "-e", "rs,toml", "-i", "src-ui/**", "-i", "crates/golden_ui/**", "-x", "run -p playground_demo" -WorkingDirectory $workspace -PassThru

Write-Host "Dev processes running. Press Ctrl+C to stop."
try {
    Wait-Process -Id $tauri.Id
} finally {
    if (!$ui.HasExited) {
        Stop-Process -Id $ui.Id
    }
}
