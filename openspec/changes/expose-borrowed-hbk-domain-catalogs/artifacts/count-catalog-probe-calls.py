#!/usr/bin/env python3
"""
GDB batch counter for OpenSpec task 1.6 borrowed-catalog probes.

Build a debug test binary first:
  RUSTFLAGS='-C opt-level=0 -C debuginfo=2 -C force-frame-pointers=yes' \
    cargo test -p context-resolver-search bsl_catalog_measurement_probe --no-run

Run current probes:
  python3 openspec/changes/expose-borrowed-hbk-domain-catalogs/artifacts/count-catalog-probe-calls.py \
    --domain bsl --binary target/debug/deps/context_resolver_search-<hash>
  python3 openspec/changes/expose-borrowed-hbk-domain-catalogs/artifacts/count-catalog-probe-calls.py \
    --domain sdbl --binary target/debug/deps/context_resolver_search-<hash>

Use `--profile baseline` for the durable baseline patch helper names, or pass
explicit marker/helper symbols. Output is deterministic JSON. The script fails
nonzero on missing required marker/helper/projection symbols, non-once marker
entry/exit, unfinished counted calls, inferior failure, or no SearchIndex
symbols. SearchIndex methods are discovered from GDB's symbol table at runtime;
there is no manual SearchIndex method list in this artifact.
"""

from __future__ import annotations

import argparse
import json
import os
import shlex
import subprocess
import sys
import tempfile
from pathlib import Path

DEFAULTS = {
    ("bsl", "current"): ("tests::bsl_catalog_measurement_probe", "context_resolver_search::tests::bsl_catalog_measurement_probe", "context_resolver_search::tests::compat_adapter_sequence", "context_resolver_search::tests::direct_bsl_catalog_sequence", "context_resolver_search::{impl#5}::global_context"),
    ("sdbl", "current"): ("tests::sdbl_catalog_measurement_probe", "context_resolver_search::tests::sdbl_catalog_measurement_probe", "context_resolver_search::tests::compat_sdbl_adapter_sequence", "context_resolver_search::tests::direct_sdbl_catalog_sequence", "context_resolver_search::{impl#7}::global_context"),
    ("bsl", "baseline"): ("tests::bsl_compat_measurement_probe", "context_resolver_search::tests::bsl_compat_measurement_probe", "context_resolver_search::tests::compat_adapter_sequence", None, "context_resolver_search::{impl#5}::global_context"),
    ("sdbl", "baseline"): ("tests::sdbl_compat_measurement_probe", "context_resolver_search::tests::sdbl_compat_measurement_probe", "context_resolver_search::tests::compat_sdbl_adapter_sequence", None, "context_resolver_search::{impl#7}::global_context"),
}

PROJECTIONS = {
    "bsl": {
        "map_platform_type": "context_resolver_search::PlatformSnapshotSource::map_platform_type",
        "map_member": "context_resolver_search::PlatformSnapshotSource::map_member",
        "map_callable": "context_resolver_search::PlatformSnapshotSource::map_callable",
        "map_global_property": "context_resolver_search::PlatformSnapshotSource::map_global_property",
        "map_event_as_member": "context_resolver_search::PlatformSnapshotSource::map_event_as_member",
        "map_availability": "context_resolver_search::PlatformSnapshotSource::map_availability",
    },
    "sdbl": {
        "map_query_table": "context_resolver_search::QueryTableSnapshotSource::map_query_table",
        "map_query_field": "context_resolver_search::QueryTableSnapshotSource::map_query_field",
        "map_query_parameter": "context_resolver_search::QueryTableSnapshotSource::map_query_parameter",
    },
}

GDB_PY = r'''
import json, os, re
import gdb

cfg = json.load(open(os.environ["CATALOG_PROBE_CONFIG"], encoding="utf-8"))
result = {"domain": cfg["domain"], "profile": cfg["profile"], "binary": cfg["binary"],
          "test_args": cfg["test_args"], "symbols": {}, "markers": {},
          "counts": {"compat": {}, "direct": {}, "outside": {}},
          "search_index": {"method_count": 0, "total_locations": 0},
          "invalid_hits": [], "inferior_exit_code": None}
mode_stack = []

def mode():
    return mode_stack[-1] if mode_stack else "outside"

def locations(bp):
    text = gdb.execute("info breakpoints " + str(bp.number), to_string=True)
    sublocs = [x.strip() for x in text.splitlines()
               if re.match(r"\s*%s\.\d+\s+" % re.escape(str(bp.number)), x)]
    if sublocs:
        return sublocs
    locs = [x.strip() for x in text.splitlines()
            if re.match(r"\s*%s\s+" % re.escape(str(bp.number)), x)]
    return locs or [x.strip() for x in text.splitlines() if x.strip()]

class Exit(gdb.FinishBreakpoint):
    def __init__(self, owner, section):
        super().__init__(internal=True)
        self.owner, self.section = owner, section
    def stop(self):
        self.owner.finish(self.section)
        return False

class Count(gdb.Breakpoint):
    def __init__(self, name, symbol, role, section=None):
        super().__init__(symbol, internal=False)
        self.name, self.symbol, self.role, self.section = name, symbol, role, section
        self.silent = True
        locs = locations(self)
        if any("<PENDING>" in loc for loc in locs):
            raise gdb.error("pending breakpoint")
        result["symbols"][name] = {"symbol": symbol, "role": role,
                                   "location_count": len(locs), "locations": locs}
    def stop(self):
        section = mode()
        if self.role == "section":
            mode_stack.append(self.section)
            section = self.section
            item = result["markers"].setdefault(self.name, {"symbol": self.symbol, "section": self.section, "entries": 0, "exits": 0})
            item["entries"] += 1
        elif self.role == "test":
            item = result["markers"].setdefault(self.name, {"symbol": self.symbol, "section": "test", "entries": 0, "exits": 0})
            item["entries"] += 1
        else:
            item = result["counts"].setdefault(section, {}).setdefault(self.name, {"entries": 0, "exits": 0})
            item["entries"] += 1
        Exit(self, section)
        return False
    def finish(self, section):
        if self.role in ("section", "test"):
            result["markers"][self.name]["exits"] += 1
            if self.role == "section":
                if not mode_stack or mode_stack[-1] != self.section:
                    result["invalid_hits"].append({"kind": "section-stack-mismatch", "name": self.name})
                else:
                    mode_stack.pop()
            return
        item = result["counts"].setdefault(section, {}).setdefault(self.name, {"entries": 0, "exits": 0})
        item["exits"] += 1

def install(name, symbol, role, section=None):
    try:
        Count(name, symbol, role, section)
    except gdb.error as exc:
        result["invalid_hits"].append({"kind": "missing-required-symbol", "name": name, "symbol": symbol, "error": str(exc)})
        print(json.dumps(result, ensure_ascii=False, sort_keys=True, indent=2))
        gdb.execute("quit 2")

def search_index_symbols():
    text = gdb.execute("info functions SearchIndex::", to_string=True)
    found = {}
    for line in text.splitlines():
        m = re.search(r"fn (syntax_helper_search::SearchIndex::[A-Za-z0-9_]+)", line)
        if m:
            symbol = m.group(1)
            found[symbol.rsplit("::", 1)[-1]] = symbol
    return found

def on_exit(event):
    result["inferior_exit_code"] = getattr(event, "exit_code", None)

def validate():
    if result["inferior_exit_code"] != 0:
        result["invalid_hits"].append({"kind": "inferior-exit-code", "exit_code": result["inferior_exit_code"]})
    if mode_stack:
        result["invalid_hits"].append({"kind": "unclosed-section-stack", "stack": list(mode_stack)})
    for name in cfg["required_markers"]:
        item = result["markers"].get(name)
        if not item or item["entries"] != 1 or item["exits"] != 1:
            result["invalid_hits"].append({"kind": "marker-count", "name": name, "value": item})
    for section, items in result["counts"].items():
        for name, item in items.items():
            if item["entries"] != item["exits"]:
                result["invalid_hits"].append({"kind": "entry-exit-mismatch", "section": section, "name": name, "value": item})

gdb.execute("set pagination off")
gdb.execute("set confirm off")
gdb.execute("set breakpoint pending off")
gdb.execute("set language rust")
gdb.execute("set inferior-tty /dev/null")
gdb.events.exited.connect(on_exit)
install("test_marker", cfg["test_marker"], "test")
install("compat_helper", cfg["compat_helper"], "section", "compat")
if cfg.get("direct_helper"):
    install("direct_helper", cfg["direct_helper"], "section", "direct")
install("worker_handle", "syntax_helper_search::snapshot::HbkFactSnapshot::worker_handle", "worker_handle")
install("context_source_global_context", cfg["context_source_global_context"], "context_source_global_context")
for name, symbol in cfg["projection_symbols"].items():
    install(name, symbol, "projection")
for name, symbol in sorted(search_index_symbols().items()):
    install("search_index::" + name, symbol, "search_index")
    result["search_index"]["method_count"] += 1
    result["search_index"]["total_locations"] += result["symbols"]["search_index::" + name]["location_count"]
if result["search_index"]["method_count"] == 0:
    raise RuntimeError("no syntax_helper_search::SearchIndex methods found in GDB symbol table")
try:
    gdb.execute("run")
except gdb.error as exc:
    result["invalid_hits"].append({"kind": "run-error", "error": str(exc)})
validate()
print(json.dumps(result, ensure_ascii=False, sort_keys=True, indent=2))
if result["invalid_hits"]:
    gdb.execute("quit 2")
'''

def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser()
    p.add_argument("--domain", choices=["bsl", "sdbl"], required=True)
    p.add_argument("--profile", choices=["current", "baseline"], default="current")
    p.add_argument("--binary", required=True)
    p.add_argument("--test-filter")
    p.add_argument("--test-marker")
    p.add_argument("--compat-helper")
    p.add_argument("--direct-helper")
    p.add_argument("--no-direct-helper", action="store_true")
    p.add_argument("--context-source-global-context")
    p.add_argument("--gdb", default="gdb")
    return p.parse_args()

def final_json(stdout: str) -> str:
    start = stdout.rfind("\n{")
    if start >= 0:
        return stdout[start + 1 :]
    start = stdout.find("{")
    return stdout[start:] if start >= 0 else stdout

def main() -> int:
    a = parse_args()
    test_filter, test_marker, compat, direct, global_context = DEFAULTS[(a.domain, a.profile)]
    if a.no_direct_helper:
        direct = None
    config = {
        "domain": a.domain, "profile": a.profile, "binary": a.binary,
        "test_args": [a.test_filter or test_filter, "--ignored", "--exact", "--nocapture"],
        "test_marker": a.test_marker or test_marker,
        "compat_helper": a.compat_helper or compat,
        "direct_helper": a.direct_helper or direct,
        "context_source_global_context": a.context_source_global_context or global_context,
        "projection_symbols": PROJECTIONS[a.domain],
        "required_markers": ["test_marker", "compat_helper"] + (["direct_helper"] if (a.direct_helper or direct) else []),
    }
    if not Path(a.binary).exists():
        raise SystemExit(f"missing binary: {a.binary}")
    with tempfile.TemporaryDirectory(prefix="catalog-probe-gdb-") as tmp:
        tmp = Path(tmp)
        cfg, script = tmp / "config.json", tmp / "probe.py"
        cfg.write_text(json.dumps(config, ensure_ascii=False, sort_keys=True), encoding="utf-8")
        script.write_text(GDB_PY, encoding="utf-8")
        env = os.environ.copy()
        env["CATALOG_PROBE_CONFIG"] = str(cfg)
        cmd = [a.gdb, "--quiet", "--batch", "--nx", "-ex", f"source {script}", "--args", a.binary, *config["test_args"]]
        print("gdb_command=" + " ".join(shlex.quote(x) for x in cmd), file=sys.stderr)
        completed = subprocess.run(cmd, env=env, check=False, stdout=subprocess.PIPE, text=True)
        sys.stdout.write(final_json(completed.stdout))
        return completed.returncode

if __name__ == "__main__":
    raise SystemExit(main())
