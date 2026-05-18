#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

command -v bun >/dev/null 2>&1 || {
  echo "bun is required for frontend dev." >&2
  exit 1
}

command -v cargo >/dev/null 2>&1 || {
  echo "cargo is required for backend dev." >&2
  exit 1
}

cargo watch --version >/dev/null 2>&1 || {
  echo "cargo-watch is required. Install it with: cargo install cargo-watch" >&2
  exit 1
}

frontend_pid=""
backend_pid=""

cleanup() {
  local status=$?
  trap - EXIT INT TERM
  if [[ -n "$frontend_pid" ]]; then kill "$frontend_pid" 2>/dev/null || true; fi
  if [[ -n "$backend_pid" ]]; then kill "$backend_pid" 2>/dev/null || true; fi
  wait "$frontend_pid" "$backend_pid" 2>/dev/null || true
  exit "$status"
}

trap cleanup EXIT INT TERM

echo "Coollabs Ideas dev mode"
echo "Open: http://127.0.0.1:5173"
echo

(
  cd "$ROOT/frontend"
  exec bun run dev
) &
frontend_pid=$!

(
  cd "$ROOT"
  if [[ -f "$ROOT/.env" ]]; then
    set -a
    # shellcheck disable=SC1091
    source "$ROOT/.env"
    set +a
  fi
  exec env SKIP_FRONTEND=1 IDEAS_SKIP_FRONTEND=1 cargo watch \
    -w crates \
    -w Cargo.toml \
    -w Cargo.lock \
    -i target \
    -x 'run -p ideas-server -- serve'
) &
backend_pid=$!

while true; do
  if ! kill -0 "$frontend_pid" 2>/dev/null; then wait "$frontend_pid"; exit $?; fi
  if ! kill -0 "$backend_pid" 2>/dev/null; then wait "$backend_pid"; exit $?; fi
  sleep 1
done
