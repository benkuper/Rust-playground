#!/usr/bin/env bash
set -euo pipefail

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
workspace="$(dirname "$root_dir")"

export GOLDEN_PORT="9010"
export VITE_GOLDEN_SERVER="http://localhost:9010"

npm --prefix "$workspace/crates/golden_ui" run dev &
ui_pid=$!

cargo watch -x "run -p golden_tauri" -C "$workspace" &
app_pid=$!

trap "kill $ui_pid $app_pid" EXIT
wait $app_pid
