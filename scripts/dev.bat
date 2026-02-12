@echo off
setlocal

set "ROOT=%~dp0"
for %%I in ("%ROOT%..") do set "WORKSPACE=%%~fI"

set "GOLDEN_PORT=9010"
set "VITE_GOLDEN_SERVER=http://localhost:9010"

echo Starting SvelteKit dev server...
start "golden-ui" cmd /k "cd /d %WORKSPACE% && npm --prefix src-ui run dev"

echo Starting Tauri app (auto-rebuild on Rust changes)...
start "golden-app" cmd /k "cd /d %WORKSPACE% && cargo watch -e rs,toml -i src-ui/** -i crates/golden_ui/** -x ""run -p playground_demo"""

echo.
echo Dev processes launched in separate terminals.
echo Close those terminal windows to stop them.

endlocal
