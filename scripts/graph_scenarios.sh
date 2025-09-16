#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Generate demo repositories with branch/merge histories to test gitup TUI graph.

Usage:
  scripts/graph_scenarios.sh scenario1 <target_dir> [--tui]
  scripts/graph_scenarios.sh scenario2 <target_dir> [--tui]

Options:
  --tui        After generating, launch gitup TUI on the repo (read $GITUP_BIN or ./target/release/gitup or PATH)

You can override the gitup binary with env var:
  GITUP_BIN=/path/to/gitup ./scripts/graph_scenarios.sh scenario1 /tmp/gitup-graph-demo --tui
USAGE
}

detect_gitup() {
  if [[ -n "${GITUP_BIN:-}" && -x "$GITUP_BIN" ]]; then
    echo "$GITUP_BIN"; return
  fi
  if [[ -x "./target/release/gitup" ]]; then
    echo "./target/release/gitup"; return
  fi
  if command -v gitup >/dev/null 2>&1; then
    command -v gitup; return
  fi
  echo "";
}

init_repo() {
  local dir="$1"
  rm -rf "$dir"
  mkdir -p "$dir"
  pushd "$dir" >/dev/null
  git init
  # Ensure main branch
  git checkout -b main >/dev/null 2>&1 || true
  # Local identity for commits
  git config user.name "Graph Demo"
  git config user.email "graph@example.com"
}

finish_repo() {
  local dir="$1"
  popd >/dev/null
  echo "Repository ready: $dir"
  git -C "$dir" --no-pager log --oneline --graph --decorate --all | sed 's/^/  /'
}

scenario1() {
  local dir="$1"
  init_repo "$dir"

  # c1, c2 on main
  echo 1 > file.txt
  git add file.txt
  git commit -m "c1 main"
  echo 2 >> main.txt
  git add main.txt
  git commit -m "c2 main (main.txt)"

  # feature branch f1, f2
  git checkout -b feature
  echo f1 > feature.txt
  git add feature.txt
  git commit -m "f1 feature (feature.txt)"
  echo f2 >> feature.txt
  git commit -am "f2 feature (feature.txt)"

  # back to main m3
  git checkout main
  echo 3 >> main.txt
  git commit -am "m3 main (main.txt)"

  # non-ff merge
  git merge --no-ff feature -m "merge feature into main (no-ff)"

  finish_repo "$dir"
}

scenario2() {
  local dir="$1"
  init_repo "$dir"

  # c1, c2
  echo a > a.txt
  git add a.txt
  git commit -m "c1 (a.txt)"
  echo a2 >> a.txt
  git commit -am "c2 (a.txt)"

  # branch feature: f1
  git checkout -b feature
  echo f1 > f.txt
  git add f.txt
  git commit -m "f1 (f.txt)"

  # back to main: m3, then merge feature
  git checkout main
  echo m3 >> a.txt
  git commit -am "m3 (a.txt)"
  git merge --no-ff feature -m "merge1 (feature->main)"

  # branch feature2: f2-1, f2-2
  git checkout -b feature2
  echo f2-1 > g.txt
  git add g.txt
  git commit -m "f2-1 (g.txt)"
  echo f2-2 >> g.txt
  git commit -am "f2-2 (g.txt)"

  # back to main: m4, then merge feature2
  git checkout main
  echo m4 >> a.txt
  git commit -am "m4 (a.txt)"
  git merge --no-ff feature2 -m "merge2 (feature2->main)"

  finish_repo "$dir"
}

run_tui() {
  local dir="$1"
  local bin
  bin="$(detect_gitup)"
  if [[ -z "$bin" ]]; then
    echo "[!] gitup binary not found. Build first: cargo build --release"
    return 1
  fi
  echo "Launching TUI: $bin tui $dir"
  "$bin" tui "$dir"
}

main() {
  if [[ $# -lt 2 ]]; then usage; exit 1; fi
  local cmd="$1"; shift
  local dir="$1"; shift
  local launch=false
  if [[ "${1:-}" == "--tui" ]]; then launch=true; shift || true; fi

  case "$cmd" in
    scenario1) scenario1 "$dir" ;;
    scenario2) scenario2 "$dir" ;;
    *) usage; exit 1 ;;
  esac

  if $launch; then run_tui "$dir"; fi
}

main "$@"
