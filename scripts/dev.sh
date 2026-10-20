#!/usr/bin/env bash
# Start steerd (http://127.0.0.1:7400) and the Vite dev server (http://localhost:5173) together;
# open http://localhost:5173. Ctrl-C stops both, and so does either one exiting (e.g. a port in use).
#
#   scripts/dev.sh
set -euo pipefail
cd "$(dirname "$0")/.."

[ -d node_modules ] || npm install
cargo build -p steerd

# `kill 0` signals this script's whole process group: steerd, npm and the vite it starts.
trap 'trap - EXIT; kill 0' EXIT INT TERM

./target/debug/steerd &
steerd=$!
npm run dev &
vite=$!

# macOS ships bash 3.2, which has no `wait -n`; poll until one of them exits.
while kill -0 "$steerd" 2>/dev/null && kill -0 "$vite" 2>/dev/null; do
  sleep 1
done
echo "dev.sh: steerd or vite exited, stopping the other" >&2
