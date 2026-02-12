#!/usr/bin/env bash
set -euo pipefail

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
workspace="$(dirname "$root_dir")"

export GOLDEN_PORT="9010"
export VITE_GOLDEN_SERVER="http://localhost:9010"

npm --prefix "$workspace/src-ui" run dev &
ui_pid=$!

cargo watch -e rs,toml -i "src-ui/**" -i "crates/golden_ui/**" -x "run -p playground_demo" -C "$workspace" &
app_pid=$!

trap "kill $ui_pid $app_pid" EXIT
wait $app_pid
