#!/usr/bin/env bash
set -u

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PLATFORM_DIR="${V8_HBK_PLATFORM_DIR:-/opt/1cv8/x86_64/8.5.1.1150}"
WORK_DIR="$ROOT_DIR/target/uat/hbk-cli-smoke"

passed=0
skipped=0
failed=0

mkdir -p "$WORK_DIR"

run_cli() {
  cargo run -q -p v8-context-hbk-cli --bin v8-context-hbk -- "$@"
}

pass_case() {
  printf 'PASS %s\n' "$1"
  passed=$((passed + 1))
}

skip_case() {
  printf 'SKIP %s: %s\n' "$1" "$2"
  skipped=$((skipped + 1))
}

fail_case() {
  printf 'FAIL %s: %s\n' "$1" "$2"
  failed=$((failed + 1))
}

first_error_line() {
  if [ -s "$1" ]; then
    head -n 1 "$1"
  else
    printf 'command failed without stderr'
  fi
}

smoke_inspect() {
  local case_id="UAT-HBK-001"
  local book="$PLATFORM_DIR/fmtdui_root.hbk"
  local out="$WORK_DIR/inspect.out"
  local err="$WORK_DIR/inspect.err"

  if [ ! -f "$book" ]; then
    skip_case "$case_id" "missing fixture $book"
    return
  fi

  if ! (cd "$ROOT_DIR" && run_cli inspect "$book" >"$out" 2>"$err"); then
    fail_case "$case_id" "$(first_error_line "$err")"
    return
  fi

  if grep -q 'PackBlock' "$out" && grep -q 'FileStorage' "$out" && grep -q 'Book' "$out"; then
    pass_case "$case_id"
  else
    fail_case "$case_id" "inspect output did not include PackBlock, FileStorage and Book"
  fi
}

smoke_toc_json() {
  local case_id="UAT-HBK-002"
  local book="$PLATFORM_DIR/fmtdui_ru.hbk"
  local out="$WORK_DIR/toc.json"
  local err="$WORK_DIR/toc.err"

  if [ ! -f "$book" ]; then
    skip_case "$case_id" "missing fixture $book"
    return
  fi

  if ! (cd "$ROOT_DIR" && run_cli toc "$book" --format json >"$out" 2>"$err"); then
    fail_case "$case_id" "$(first_error_line "$err")"
    return
  fi

  if python3 - "$out" <<'PY'
import json
import sys

with open(sys.argv[1], "r", encoding="utf-8-sig") as fh:
    data = json.load(fh)

def walk(nodes):
    for node in nodes:
        yield node
        yield from walk(node.get("children", []))

if not isinstance(data, list):
    raise SystemExit("TOC JSON root is not an array")

if not any(node.get("html_path") and isinstance(node.get("title"), dict) for node in walk(data)):
    raise SystemExit("TOC JSON has no item with title and html_path")
PY
  then
    pass_case "$case_id"
  else
    fail_case "$case_id" "toc output was not valid JSON with title/html_path items"
  fi
}

smoke_page() {
  local case_id="UAT-HBK-003"
  local book="$PLATFORM_DIR/fmtdui_ru.hbk"
  local page_fixture="$ROOT_DIR/tests/fixtures/known-pages/fmtdui_ru.page"
  local out="$WORK_DIR/page.html"
  local err="$WORK_DIR/page.err"
  local page_path

  if [ ! -f "$book" ]; then
    skip_case "$case_id" "missing fixture $book"
    return
  fi
  if [ ! -f "$page_fixture" ]; then
    skip_case "$case_id" "missing fixture $page_fixture"
    return
  fi

  page_path="$(tr -d '\r\n' <"$page_fixture")"
  if [ -z "$page_path" ]; then
    fail_case "$case_id" "known page fixture is empty"
    return
  fi

  if ! (cd "$ROOT_DIR" && run_cli page "$book" --path "$page_path" >"$out" 2>"$err"); then
    fail_case "$case_id" "$(first_error_line "$err")"
    return
  fi

  if [ -s "$out" ] && grep -qi '<html' "$out" && grep -qi '<body' "$out"; then
    pass_case "$case_id"
  else
    fail_case "$case_id" "page output was empty or did not look like HBK page HTML"
  fi
}

smoke_inspect
smoke_toc_json
smoke_page

printf 'summary: passed=%s skipped=%s failed=%s\n' "$passed" "$skipped" "$failed"

if [ "$failed" -gt 0 ]; then
  exit 1
fi
