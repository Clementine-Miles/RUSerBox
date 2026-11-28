#!/usr/bin/env bash
# Rebuild the Monitor agent on a development host and restart its user service.
set -euo pipefail

[ $# -eq 1 ] || { echo "usage: $0 <ssh host>" >&2; exit 2; }
host=$1
root=$(cd "$(dirname "$0")/.." && pwd)
src='src/ruserbox'
app='apps/server-box-monitor'
# The remote login shell may be fish: every remote command is sh's, on stdin.
remote() { printf '%s\n' "$1" | ssh "$host" sh -s; }

echo "==> agent: sync source to $host:~/$src"
rsync -az --delete \
  --exclude target --exclude node_modules --exclude build --exclude .git \
  --exclude '.claude/worktrees' \
  "$root/" "$host:$src/"
echo "==> agent: build (monitor-test)"
remote ". \"\$HOME/.cargo/env\" 2>/dev/null; cd ~/$src && cargo build -p server_box_monitor --profile monitor-test"

remote "systemctl --user stop server-box-monitor"
remote "cp ~/$src/target/monitor-test/server_box_monitor ~/$app/server_box_monitor.new && mv ~/$app/server_box_monitor.new ~/$app/server_box_monitor"
remote "systemctl --user start server-box-monitor && sleep 3 && systemctl --user is-active server-box-monitor"
