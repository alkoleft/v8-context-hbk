#!/usr/bin/env python3
"""Run the isolated S83-AV1 availability enumeration evidence matrix.

The backend manifest is a JSON object with a ``backends`` array.  Every entry
contains ``backend``, ``decision_role``, ``worktree``, ``command`` (an argv
array containing both ``{context}`` and ``{iterations}``) and one or more
``declared_files``.  Commands are never passed through a shell.

Example backend entry::

    {
      "backend": "S83-H0",
      "decision_role": "baseline",
      "worktree": "/path/to/v8-context-hbk",
      "command": ["target/release/examples/measure_hbk_s83_av1",
                  "sql-owned", "target/provider.sqlite",
                  "{context}", "{iterations}"],
      "declared_files": ["target/provider.sqlite"]
    }

The driver first obtains and stores the canonical H0 transcript for every
context, then proves every other backend against it.  It starts timing only
after the complete parity matrix has passed.  All child processes run
sequentially on the shared host.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import re
import socket
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable, Sequence


REPORT_SCHEMA = "hbk-s83-av1-benchmark/v1"
WORKLOAD_VERSION = "s83-av1-filtered-global-method-enumeration/v1"
RAW_SCHEMA = "hbk-s83-av1-raw/v1"
PARITY_SCHEMA = "hbk-s83-av1-parity/v1"
ORCHESTRATION_VERSION = "hbk-s83-av1-orchestration/v1"

DATASET = "shcntx_ru-8.3.27.1859-schema16-extraction11"
PLATFORM_VERSION = "8.3.27.1859"
SOURCE_LOCALE = "ru"
PROVIDER_SCHEMA_VERSION = 16
EXTRACTION_SCHEMA_VERSION = 11
HBK_BYTES = 40_744_845
HBK_SHA256 = "5bdf0b3ed89932572c012faddc4d05ebfa2986595cf2849b6eb6e5e65a9a4d48"
PROVIDER_BYTES = 204_288_000
PROVIDER_SHA256 = "55c2e09971712a13a49cbcf5889f203d7a9dfcec22aa0d333247ae722f6f0fab"

AVAILABILITY_CONTEXTS = (
    "thin_client",
    "web_client",
    "mobile_client",
    "server",
    "thick_client",
    "external_connection",
    "mobile_application_client",
    "mobile_application_server",
    "mobile_standalone_server",
)
CACHE_STANCES = ("warm", "cold-best-effort")
DEFAULT_SAMPLES = 9
DEFAULT_ITERATIONS = 1_000
DEFAULT_PARITY_ITERATIONS = 1
BACKEND_PATTERN = re.compile(r"^[A-Za-z0-9._-]+$")
FORBIDDEN_METADATA_KEY = re.compile(r"module_?context_?kind", re.IGNORECASE)
HARNESS_PATHS = (
    "crates/syntax-helper-search/Cargo.toml",
    "crates/syntax-helper-search/examples/measure_hbk_s83_av1.rs",
    "scripts/benchmark-hbk-s83-av1.py",
    "scripts/summarize-hbk-s83-av1-results.py",
)
REPORT_KEYS = frozenset(
    {
        "schema_version",
        "workload_version",
        "mode",
        "backend",
        "decision_role",
        "baseline_role",
        "module_context_filter_used",
        "empty_availability_rule",
        "availability_context",
        "iterations",
        "input_identity",
        "index",
        "cache",
        "cache_status",
        "counts",
        "timings",
        "allocations",
        "transcript",
    }
)
COUNT_KEYS = frozenset(
    {
        "scanned_globals",
        "candidate_methods",
        "returned_objects",
        "universal_objects",
        "explicit_context_objects",
        "excluded_objects",
        "universal_assertion",
        "excluded_assertion",
    }
)
TIMING_KEYS = frozenset(
    {
        "phase_order",
        "entry_to_ready_ns",
        "open",
        "first_enumeration",
        "warmup",
        "workload",
    }
)
FAULT_KEYS = frozenset({"minor", "major"})
PHASE_KEYS = frozenset({"elapsed_ns", "faults"})
ENUMERATION_PHASE_KEYS = frozenset(
    {"elapsed_ns", "ns_per_object", "faults", "returned_objects", "checksum"}
)
WORKLOAD_KEYS = frozenset(
    {
        "elapsed_ns",
        "average_ns",
        "ns_per_object",
        "faults",
        "iterations",
        "returned_total",
        "checksum",
    }
)
ALLOCATIONS_KEYS = frozenset(
    {
        "enabled",
        "entry_to_ready",
        "first_enumeration",
        "warmup",
        "workload",
        "final_snapshot",
    }
)
ALLOCATION_DELTA_KEYS = frozenset(
    {
        "allocation_calls",
        "reallocation_calls",
        "deallocation_calls",
        "allocated_bytes",
        "deallocated_bytes",
        "live_bytes_before",
        "live_bytes_after",
        "peak_live_bytes_before",
        "peak_live_bytes_after",
        "peak_live_bytes_growth",
    }
)
ALLOCATION_SNAPSHOT_KEYS = frozenset(
    {
        "allocation_calls",
        "reallocation_calls",
        "deallocation_calls",
        "allocated_bytes",
        "deallocated_bytes",
        "current_live_bytes",
        "peak_live_bytes",
    }
)


class EvidenceError(RuntimeError):
    """An evidence contract violation."""


@dataclass(frozen=True)
class Backend:
    backend: str
    decision_role: str
    worktree: Path
    command: tuple[str, ...]
    declared_files: tuple[Path, ...]
    declared_file_artifacts: tuple[dict[str, Any], ...]
    commit: str
    branch: str


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, default=Path.cwd())
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--results-root", type=Path, required=True)
    parser.add_argument("--samples", type=positive_int, default=DEFAULT_SAMPLES)
    parser.add_argument("--iterations", type=positive_int, default=DEFAULT_ITERATIONS)
    parser.add_argument(
        "--parity-iterations", type=positive_int, default=DEFAULT_PARITY_ITERATIONS
    )
    parser.add_argument("--timeout-seconds", type=positive_int, default=900)
    return parser.parse_args(argv)


def positive_int(value: str) -> int:
    parsed = int(value)
    if parsed <= 0:
        raise argparse.ArgumentTypeError("value must be greater than zero")
    return parsed


def run_text(command: Sequence[str], cwd: Path | None = None) -> str:
    return subprocess.check_output(command, cwd=cwd, text=True).strip()


def git_text(worktree: Path, *args: str) -> str:
    return run_text(("git", "-C", str(worktree), *args))


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def artifact_identity(path: Path) -> dict[str, Any]:
    resolved = path.resolve()
    return {
        "path": str(resolved),
        "bytes": resolved.stat().st_size,
        "sha256": sha256_file(resolved),
    }


def canonical_json_bytes(value: Any) -> bytes:
    return json.dumps(
        value, ensure_ascii=False, sort_keys=True, separators=(",", ":")
    ).encode("utf-8")


def command_for(template: Sequence[str], context: str, iterations: int) -> list[str]:
    command = [
        part.replace("{context}", context).replace("{iterations}", str(iterations))
        for part in template
    ]
    if any("{context}" in part or "{iterations}" in part for part in command):
        raise EvidenceError("unexpanded command placeholder")
    return command


def resolve_path(worktree: Path, raw: str) -> Path:
    path = Path(raw)
    return path if path.is_absolute() else worktree / path


def command_file_arguments(backend: Backend, context: str, iterations: int) -> list[Path]:
    files: list[Path] = []
    for raw in command_for(backend.command, context, iterations)[1:]:
        path = resolve_path(backend.worktree, raw).resolve()
        if path.is_file():
            files.append(path)
    return files


def load_backends(manifest_path: Path) -> tuple[list[Backend], str]:
    raw_bytes = manifest_path.read_bytes()
    value = json.loads(raw_bytes)
    entries = value.get("backends") if isinstance(value, dict) else None
    if not isinstance(entries, list) or not entries:
        raise EvidenceError("manifest must contain a non-empty backends array")

    backends: list[Backend] = []
    seen: set[str] = set()
    for index, entry in enumerate(entries):
        if not isinstance(entry, dict):
            raise EvidenceError(f"backends[{index}] must be an object")
        backend = entry.get("backend")
        role = entry.get("decision_role")
        raw_worktree = entry.get("worktree")
        command = entry.get("command")
        raw_files = entry.get("declared_files")
        if not isinstance(backend, str) or not BACKEND_PATTERN.fullmatch(backend):
            raise EvidenceError(f"backends[{index}] has an invalid backend label")
        if backend in seen:
            raise EvidenceError(f"duplicate backend in manifest: {backend}")
        if not isinstance(raw_worktree, str):
            raise EvidenceError(f"{backend}: worktree must be a string")
        if (
            not isinstance(command, list)
            or not command
            or any(not isinstance(part, str) or not part for part in command)
        ):
            raise EvidenceError(f"{backend}: command must be a non-empty argv array")
        if not any("{context}" in part for part in command) or not any(
            "{iterations}" in part for part in command
        ):
            raise EvidenceError(
                f"{backend}: command must contain {{context}} and {{iterations}}"
            )
        if (
            not isinstance(raw_files, list)
            or not raw_files
            or any(not isinstance(path, str) or not path for path in raw_files)
        ):
            raise EvidenceError(f"{backend}: declared_files must be non-empty")

        expected_role = (
            "baseline" if backend == "S83-H0" else "control" if backend == "S83-C0" else "candidate"
        )
        if role != expected_role:
            raise EvidenceError(
                f"{backend}: decision_role must be {expected_role!r}, got {role!r}"
            )
        worktree = Path(raw_worktree).resolve()
        if git_text(worktree, "status", "--porcelain"):
            raise EvidenceError(f"backend worktree is dirty: {worktree}")
        files = tuple(resolve_path(worktree, path).resolve() for path in raw_files)
        if len(files) != len(set(files)):
            raise EvidenceError(f"{backend}: declared_files contains duplicate paths")
        for path in files:
            if not path.is_file():
                raise EvidenceError(f"{backend}: declared file does not exist: {path}")
        file_artifacts = tuple(artifact_identity(path) for path in files)
        backends.append(
            Backend(
                backend=backend,
                decision_role=role,
                worktree=worktree,
                command=tuple(command),
                declared_files=files,
                declared_file_artifacts=file_artifacts,
                commit=git_text(worktree, "rev-parse", "HEAD"),
                branch=git_text(worktree, "branch", "--show-current"),
            )
        )
        seen.add(backend)

    if "S83-H0" not in seen or "S83-C0" not in seen:
        raise EvidenceError("manifest must contain S83-H0 baseline and S83-C0 control")
    by_name = {backend.backend: backend for backend in backends}
    ordered = [by_name["S83-H0"], by_name["S83-C0"]]
    ordered.extend(backend for backend in backends if backend.backend not in {"S83-H0", "S83-C0"})
    return ordered, sha256_bytes(raw_bytes)


def verify_harness(repo: Path) -> tuple[str, str, dict[str, str]]:
    repo = repo.resolve()
    dirty = git_text(repo, "status", "--porcelain", "--", *HARNESS_PATHS)
    if dirty:
        raise EvidenceError(f"S83-AV1 harness has uncommitted changes:\n{dirty}")
    hashes: dict[str, str] = {}
    for relative in HARNESS_PATHS:
        path = repo / relative
        if not path.is_file():
            raise EvidenceError(f"missing S83-AV1 harness file: {path}")
        hashes[relative] = sha256_file(path)
    # The HEAD tree freezes the complete harness even when its files were
    # introduced by more than one preparatory commit. Per-file hashes make the
    # exact executable/script set explicit in every raw record.
    harness_commit = git_text(repo, "rev-parse", "HEAD")
    return harness_commit, git_text(repo, "branch", "--show-current"), hashes


def host_environment() -> dict[str, Any]:
    return {
        "hostname": socket.gethostname(),
        "kernel": f"{platform.system()} {platform.release()}",
        "architecture": platform.machine(),
        "python": platform.python_version(),
        "rustc": run_text(("rustc", "--version")),
        "cargo": run_text(("cargo", "--version")),
        "logical_cpus": os.cpu_count(),
    }


def machine_state() -> dict[str, Any]:
    load_one, load_five, load_fifteen, scheduler_tasks, last_pid = (
        Path("/proc/loadavg").read_text(encoding="ascii").split()
    )
    runnable, total = scheduler_tasks.split("/", 1)
    memory: dict[str, int] = {}
    wanted = {
        "MemAvailable": "available_kib",
        "MemFree": "free_kib",
        "Cached": "cached_kib",
        "SwapFree": "swap_free_kib",
        "Dirty": "dirty_kib",
    }
    for line in Path("/proc/meminfo").read_text(encoding="ascii").splitlines():
        name, value, *_unit = line.replace(":", "").split()
        if name in wanted:
            memory[wanted[name]] = int(value)
    return {
        "captured_unix_ns": time.time_ns(),
        "load_average": {
            "one_minute": float(load_one),
            "five_minutes": float(load_five),
            "fifteen_minutes": float(load_fifteen),
        },
        "scheduler": {
            "runnable_tasks": int(runnable),
            "total_tasks": int(total),
            "last_pid": int(last_pid),
        },
        "memory": memory,
    }


def prepare_files(stance: str, paths: Iterable[Path]) -> dict[str, Any]:
    files = [str(path) for path in paths]
    if stance == "warm":
        for raw_path in files:
            with Path(raw_path).open("rb", buffering=0) as stream:
                while stream.read(8 * 1024 * 1024):
                    pass
        return {"method": "read-declared-files", "declared_files": files}
    if stance != "cold-best-effort":
        raise EvidenceError(f"unknown cache stance: {stance}")
    if not hasattr(os, "posix_fadvise") or not hasattr(os, "POSIX_FADV_DONTNEED"):
        raise EvidenceError("cold-best-effort requires os.posix_fadvise(POSIX_FADV_DONTNEED)")
    subprocess.run(("sync",), check=True)
    for raw_path in files:
        fd = os.open(raw_path, os.O_RDONLY)
        try:
            os.posix_fadvise(fd, 0, 0, os.POSIX_FADV_DONTNEED)
        finally:
            os.close(fd)
    return {
        "method": "sync+posix_fadvise-dontneed",
        "declared_files": files,
        "eviction_verified": False,
        "claim": "cold-best-effort",
    }


def forbidden_metadata_paths(value: Any, prefix: str = "") -> list[str]:
    matches: list[str] = []
    if isinstance(value, dict):
        for key, nested in value.items():
            path = f"{prefix}.{key}" if prefix else str(key)
            if key != "module_context_filter_used" and FORBIDDEN_METADATA_KEY.search(str(key)):
                matches.append(path)
            matches.extend(forbidden_metadata_paths(nested, path))
    elif isinstance(value, list):
        for index, nested in enumerate(value):
            matches.extend(forbidden_metadata_paths(nested, f"{prefix}[{index}]"))
    return matches


def normalize_artifact(
    value: dict[str, Any], worktree: Path, label: str
) -> tuple[str, int, str]:
    raw_path = value.get("path")
    raw_bytes = value.get("bytes")
    raw_sha256 = value.get("sha256")
    if not isinstance(raw_path, str):
        raise EvidenceError(f"{label}.path must be a string")
    if isinstance(raw_bytes, bool) or not isinstance(raw_bytes, int) or raw_bytes <= 0:
        raise EvidenceError(f"{label}.bytes must be a positive integer: {raw_path}")
    if (
        not isinstance(raw_sha256, str)
        or not re.fullmatch(r"[0-9a-f]{64}", raw_sha256)
    ):
        raise EvidenceError(f"{label}.sha256 must be a lowercase SHA-256: {raw_path}")
    path = resolve_path(worktree, raw_path).resolve()
    return (str(path), raw_bytes, raw_sha256)


def runtime_artifacts(
    report: dict[str, Any], backend: Backend
) -> list[dict[str, Any]]:
    index = report.get("index")
    cache = report.get("cache")
    cache_status = report.get("cache_status")
    if backend.backend == "S83-H0":
        if not isinstance(index, dict) or cache is not None or cache_status is not None:
            raise EvidenceError(
                "S83-H0 runtime artifact contract requires index only"
            )
        return [index]
    if backend.backend == "S83-C0":
        if (
            not isinstance(index, dict)
            or not isinstance(cache, dict)
            or cache_status != "loaded"
        ):
            raise EvidenceError(
                "S83-C0 runtime artifact contract requires index + loaded cache"
            )
        return [index, cache]
    if isinstance(cache, dict):
        if cache_status != "loaded":
            raise EvidenceError(
                f"{backend.backend} runtime artifact contract requires loaded candidate cache"
            )
        return [cache]
    raise EvidenceError(
        f"{backend.backend} runtime artifact contract requires one loaded candidate cache"
    )


def validate_declared_artifact_binding(
    report: dict[str, Any],
    backend: Backend,
    context: str,
    iterations: int,
) -> None:
    declared_values = [
        normalize_artifact(value, backend.worktree, "declared_file_artifacts")
        for value in backend.declared_file_artifacts
    ]
    observed_values = [
        normalize_artifact(value, backend.worktree, "runtime artifact")
        for value in runtime_artifacts(report, backend)
    ]
    if len(declared_values) != len(set(declared_values)):
        raise EvidenceError(f"{backend.backend}: duplicate declared runtime artifacts")
    if len(observed_values) != len(set(observed_values)):
        raise EvidenceError(f"{backend.backend}: duplicate reported runtime artifacts")
    declared = set(declared_values)
    observed = set(observed_values)
    if declared != observed:
        raise EvidenceError(
            f"{backend.backend}/{context}: declared artifacts do not exactly match "
            f"runtime artifacts: declared={sorted(declared)}, observed={sorted(observed)}"
        )
    declared_paths = {Path(path) for path, _bytes, _sha in declared}
    extra_file_args = [
        str(path)
        for path in command_file_arguments(backend, context, iterations)
        if path not in declared_paths
    ]
    if extra_file_args:
        raise EvidenceError(
            f"{backend.backend}/{context}: command references file(s) outside declared_files: "
            f"{extra_file_args}"
        )


def require_integer(value: Any, path: str, *, positive: bool = False) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        raise EvidenceError(f"{path} must be an integer")
    if positive and value <= 0:
        raise EvidenceError(f"{path} must be greater than zero")
    return value


def require_object(value: Any, path: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise EvidenceError(f"{path} must be an object")
    return value


def require_exact_keys(
    value: dict[str, Any], expected: frozenset[str], path: str
) -> None:
    actual = set(value)
    if actual != expected:
        raise EvidenceError(
            f"{path} schema keys differ: missing={sorted(expected - actual)}, "
            f"unexpected={sorted(actual - expected)}"
        )


def validate_faults(value: Any, path: str) -> dict[str, Any]:
    faults = require_object(value, path)
    require_exact_keys(faults, FAULT_KEYS, path)
    require_integer(faults.get("minor"), f"{path}.minor")
    require_integer(faults.get("major"), f"{path}.major")
    return faults


def validate_phase(value: Any, path: str) -> dict[str, Any]:
    phase = require_object(value, path)
    require_exact_keys(phase, PHASE_KEYS, path)
    require_integer(phase.get("elapsed_ns"), f"{path}.elapsed_ns", positive=True)
    validate_faults(phase.get("faults"), f"{path}.faults")
    return phase


def validate_enumeration_phase(
    value: Any, path: str, expected_returned_objects: int
) -> dict[str, Any]:
    phase = require_object(value, path)
    require_exact_keys(phase, ENUMERATION_PHASE_KEYS, path)
    require_integer(phase.get("elapsed_ns"), f"{path}.elapsed_ns", positive=True)
    ns_per_object = phase.get("ns_per_object")
    if ns_per_object is not None:
        require_integer(ns_per_object, f"{path}.ns_per_object")
    validate_faults(phase.get("faults"), f"{path}.faults")
    if phase.get("returned_objects") != expected_returned_objects:
        raise EvidenceError(f"{path}.returned_objects differs from transcript")
    require_integer(phase.get("checksum"), f"{path}.checksum")
    return phase


def validate_workload(
    value: Any, path: str, iterations: int, returned_objects: int
) -> dict[str, Any]:
    workload = require_object(value, path)
    require_exact_keys(workload, WORKLOAD_KEYS, path)
    require_integer(workload.get("elapsed_ns"), f"{path}.elapsed_ns", positive=True)
    require_integer(workload.get("average_ns"), f"{path}.average_ns", positive=True)
    ns_per_object = workload.get("ns_per_object")
    if ns_per_object is not None:
        require_integer(ns_per_object, f"{path}.ns_per_object")
    validate_faults(workload.get("faults"), f"{path}.faults")
    if workload.get("iterations") != iterations:
        raise EvidenceError("workload iteration count differs from command")
    if workload.get("returned_total") != iterations * returned_objects:
        raise EvidenceError("workload returned total is inconsistent")
    require_integer(workload.get("checksum"), f"{path}.checksum")
    return workload


def validate_allocation_delta(value: Any, path: str) -> None:
    delta = require_object(value, path)
    require_exact_keys(delta, ALLOCATION_DELTA_KEYS, path)
    for key in ALLOCATION_DELTA_KEYS:
        require_integer(delta.get(key), f"{path}.{key}")


def validate_allocation_snapshot(value: Any, path: str) -> None:
    snapshot = require_object(value, path)
    require_exact_keys(snapshot, ALLOCATION_SNAPSHOT_KEYS, path)
    for key in ALLOCATION_SNAPSHOT_KEYS:
        require_integer(snapshot.get(key), f"{path}.{key}")


def validate_report(
    report: Any,
    backend: Backend,
    context: str,
    iterations: int,
    *,
    require_allocations: bool,
) -> tuple[bytes, dict[str, Any]]:
    report = require_object(report, "report")
    forbidden = forbidden_metadata_paths({key: value for key, value in report.items() if key != "transcript"})
    if forbidden:
        raise EvidenceError(
            f"{backend.backend}/{context}: ModuleContextKind metadata is forbidden: {forbidden}"
        )
    require_exact_keys(report, REPORT_KEYS, "report")
    validate_declared_artifact_binding(report, backend, context, iterations)
    expected = {
        "schema_version": REPORT_SCHEMA,
        "workload_version": WORKLOAD_VERSION,
        "backend": backend.backend,
        "decision_role": backend.decision_role,
        "baseline_role": "h0",
        "module_context_filter_used": False,
        "empty_availability_rule": "universal",
        "availability_context": context,
        "iterations": iterations,
    }
    for key, expected_value in expected.items():
        if report.get(key) != expected_value:
            raise EvidenceError(
                f"{backend.backend}/{context}: {key} expected {expected_value!r}, "
                f"got {report.get(key)!r}"
            )
    identity = require_object(report.get("input_identity"), "input_identity")
    identity_expected = {
        "platform_version": PLATFORM_VERSION,
        "source_locale": SOURCE_LOCALE,
        "provider_schema_version": PROVIDER_SCHEMA_VERSION,
        "extraction_schema_version": EXTRACTION_SCHEMA_VERSION,
    }
    for key, expected_value in identity_expected.items():
        if identity.get(key) != expected_value:
            raise EvidenceError(f"input_identity.{key} does not identify frozen S83")
    for key, expected_bytes, expected_sha in (
        ("hbk", HBK_BYTES, HBK_SHA256),
        ("provider", PROVIDER_BYTES, PROVIDER_SHA256),
    ):
        artifact = require_object(identity.get(key), f"input_identity.{key}")
        if artifact.get("bytes") != expected_bytes or artifact.get("sha256") != expected_sha:
            raise EvidenceError(f"input_identity.{key} does not identify frozen S83")

    counts = require_object(report.get("counts"), "counts")
    require_exact_keys(counts, COUNT_KEYS, "counts")
    for field in (
        "scanned_globals",
        "candidate_methods",
        "returned_objects",
        "universal_objects",
        "explicit_context_objects",
        "excluded_objects",
    ):
        require_integer(counts.get(field), f"counts.{field}", positive=True)
    if counts.get("universal_assertion") is not True or counts.get("excluded_assertion") is not True:
        raise EvidenceError(f"{backend.backend}/{context}: S83 universal/excluded guard failed")
    if counts["candidate_methods"] != counts["returned_objects"] + counts["excluded_objects"]:
        raise EvidenceError(f"{backend.backend}/{context}: inconsistent candidate count")
    if counts["returned_objects"] != counts["universal_objects"] + counts["explicit_context_objects"]:
        raise EvidenceError(f"{backend.backend}/{context}: inconsistent returned count")

    transcript = report.get("transcript")
    if not isinstance(transcript, list):
        raise EvidenceError("transcript must be an array")
    if len(transcript) != counts["returned_objects"]:
        raise EvidenceError(
            f"{backend.backend}/{context}: transcript/count mismatch "
            f"({len(transcript)} != {counts['returned_objects']})"
        )

    timings = require_object(report.get("timings"), "timings")
    require_exact_keys(timings, TIMING_KEYS, "timings")
    if timings.get("phase_order") != ["entry_to_ready", "first_enumeration", "warmup", "workload"]:
        raise EvidenceError("unexpected timing phase order")
    require_integer(timings.get("entry_to_ready_ns"), "timings.entry_to_ready_ns", positive=True)
    validate_phase(timings.get("open"), "timings.open")
    validate_enumeration_phase(
        timings.get("first_enumeration"),
        "timings.first_enumeration",
        counts["returned_objects"],
    )
    validate_enumeration_phase(
        timings.get("warmup"),
        "timings.warmup",
        counts["returned_objects"],
    )
    validate_workload(
        timings.get("workload"),
        "timings.workload",
        iterations,
        counts["returned_objects"],
    )

    allocations = require_object(report.get("allocations"), "allocations")
    require_exact_keys(allocations, ALLOCATIONS_KEYS, "allocations")
    if require_allocations and allocations.get("enabled") is not True:
        raise EvidenceError("timed S83-AV1 evidence requires allocation instrumentation")
    if not isinstance(allocations.get("enabled"), bool):
        raise EvidenceError("allocations.enabled must be boolean")
    for phase in ("entry_to_ready", "first_enumeration", "warmup", "workload"):
        validate_allocation_delta(allocations.get(phase), f"allocations.{phase}")
    validate_allocation_snapshot(allocations.get("final_snapshot"), "allocations.final_snapshot")

    transcript_bytes = canonical_json_bytes(transcript)
    stripped = dict(report)
    del stripped["transcript"]
    return transcript_bytes, stripped


def execute_report(
    backend: Backend,
    context: str,
    iterations: int,
    timeout_seconds: int,
    stdout_path: Path,
    stderr_path: Path,
) -> Any:
    command = command_for(backend.command, context, iterations)
    completed = subprocess.run(
        command,
        cwd=backend.worktree,
        text=True,
        capture_output=True,
        timeout=timeout_seconds,
        check=False,
    )
    stdout_path.parent.mkdir(parents=True, exist_ok=True)
    stdout_path.write_text(completed.stdout, encoding="utf-8")
    stderr_path.write_text(completed.stderr, encoding="utf-8")
    if completed.returncode != 0:
        raise EvidenceError(
            f"{backend.backend}/{context}: command failed with {completed.returncode}; "
            f"see {stderr_path}"
        )
    try:
        return json.loads(completed.stdout)
    except json.JSONDecodeError as error:
        raise EvidenceError(
            f"{backend.backend}/{context}: stdout is not one JSON report; see {stdout_path}"
        ) from error


def append_jsonl(path: Path, record: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a", encoding="utf-8") as stream:
        json.dump(record, stream, ensure_ascii=False, separators=(",", ":"))
        stream.write("\n")
        stream.flush()
        os.fsync(stream.fileno())


def identity_fields(
    backend: Backend,
    harness_commit: str,
    harness_branch: str,
    harness_hashes: dict[str, str],
    manifest_sha256: str,
    host: dict[str, Any],
) -> dict[str, Any]:
    return {
        "dataset": DATASET,
        "platform_version": PLATFORM_VERSION,
        "provider_schema_version": PROVIDER_SCHEMA_VERSION,
        "extraction_schema_version": EXTRACTION_SCHEMA_VERSION,
        "hbk_sha256": HBK_SHA256,
        "provider_sha256": PROVIDER_SHA256,
        "backend": backend.backend,
        "decision_role": backend.decision_role,
        "baseline_role": "h0",
        "candidate_commit": backend.commit,
        "candidate_branch": backend.branch,
        "worktree": str(backend.worktree),
        "harness_commit": harness_commit,
        "harness_branch": harness_branch,
        "harness_file_sha256": harness_hashes,
        "manifest_sha256": manifest_sha256,
        "host": host,
        "orchestration_version": ORCHESTRATION_VERSION,
        "declared_file_artifacts": [
            dict(artifact) for artifact in backend.declared_file_artifacts
        ],
    }


def transcript_evidence(transcript_bytes: bytes, baseline_bytes: bytes) -> dict[str, Any]:
    return {
        "sha256": sha256_bytes(transcript_bytes),
        "bytes": len(transcript_bytes),
        "item_count": len(json.loads(transcript_bytes)),
        "baseline_sha256": sha256_bytes(baseline_bytes),
        "parity_status": "pass" if transcript_bytes == baseline_bytes else "mismatch",
    }


def parity_phase(
    backends: Sequence[Backend],
    results_root: Path,
    parity_iterations: int,
    timeout_seconds: int,
    base_identity: dict[str, dict[str, Any]],
) -> dict[str, bytes]:
    parity_jsonl = results_root / "parity" / "parity.jsonl"
    if parity_jsonl.exists():
        raise EvidenceError(f"refusing to append to existing parity evidence: {parity_jsonl}")
    baseline: dict[str, bytes] = {}
    for backend in backends:
        for context in AVAILABILITY_CONTEXTS:
            slug = f"{backend.backend}-{context}"
            stdout_path = results_root / "logs" / "parity" / f"{slug}.stdout.json"
            stderr_path = results_root / "logs" / "parity" / f"{slug}.stderr.log"
            report = execute_report(
                backend,
                context,
                parity_iterations,
                timeout_seconds,
                stdout_path,
                stderr_path,
            )
            transcript, stripped = validate_report(
                report,
                backend,
                context,
                parity_iterations,
                require_allocations=False,
            )
            if backend.backend == "S83-H0":
                baseline[context] = transcript
                transcript_path = results_root / "parity" / "h0" / f"{context}.transcript.json"
                transcript_path.parent.mkdir(parents=True, exist_ok=True)
                transcript_path.write_bytes(transcript + b"\n")
            baseline_transcript = baseline.get(context)
            if baseline_transcript is None:
                raise EvidenceError("internal error: H0 parity must run first")
            evidence = transcript_evidence(transcript, baseline_transcript)
            stdout_path.write_bytes(
                canonical_json_bytes(
                    {"measurement": stripped, "transcript": evidence}
                )
                + b"\n"
            )
            record = {
                "schema": PARITY_SCHEMA,
                **base_identity[backend.backend],
                "availability_context": context,
                "module_context_filter_used": False,
                "empty_availability_rule": "universal",
                "iterations": parity_iterations,
                "command_template": list(backend.command),
                "command": command_for(backend.command, context, parity_iterations),
                "declared_file_artifacts": list(backend.declared_file_artifacts),
                "stdout_log": str(stdout_path),
                "stderr_log": str(stderr_path),
                "transcript": evidence,
            }
            append_jsonl(parity_jsonl, record)
            if evidence["parity_status"] != "pass":
                raise EvidenceError(
                    f"{backend.backend}/{context}: ordered transcript differs from S83-H0"
                )
    return baseline


def measurement_phase(
    backends: Sequence[Backend],
    results_root: Path,
    baseline: dict[str, bytes],
    samples: int,
    iterations: int,
    timeout_seconds: int,
    base_identity: dict[str, dict[str, Any]],
) -> None:
    raw_path = results_root / "raw" / "measurements.jsonl"
    if raw_path.exists():
        raise EvidenceError(f"refusing to append to existing raw evidence: {raw_path}")
    # Deliberately nested and blocking: there is never more than one child process.
    for backend in backends:
        for context in AVAILABILITY_CONTEXTS:
            for stance in CACHE_STANCES:
                for sample in range(1, samples + 1):
                    preparation = prepare_files(stance, backend.declared_files)
                    before = machine_state()
                    slug = f"{backend.backend}-{context}-{stance}-{sample:02d}"
                    stdout_path = results_root / "logs" / "measurements" / f"{slug}.stdout.json"
                    stderr_path = results_root / "logs" / "measurements" / f"{slug}.stderr.log"
                    report = execute_report(
                        backend,
                        context,
                        iterations,
                        timeout_seconds,
                        stdout_path,
                        stderr_path,
                    )
                    after = machine_state()
                    transcript, stripped = validate_report(
                        report,
                        backend,
                        context,
                        iterations,
                        require_allocations=True,
                    )
                    transcript_meta = transcript_evidence(transcript, baseline[context])
                    if transcript_meta["parity_status"] != "pass":
                        raise EvidenceError(
                            f"{backend.backend}/{context}/{stance}/{sample}: "
                            "ordered transcript differs from S83-H0"
                        )
                    stdout_path.write_bytes(
                        canonical_json_bytes(
                            {"measurement": stripped, "transcript": transcript_meta}
                        )
                        + b"\n"
                    )
                    record = {
                        "schema": RAW_SCHEMA,
                        **base_identity[backend.backend],
                        "availability_context": context,
                        "module_context_filter_used": False,
                        "empty_availability_rule": "universal",
                        "cache_stance": stance,
                        "sample": sample,
                        "iterations": iterations,
                        "status": "ok",
                        "command_template": list(backend.command),
                        "command": command_for(backend.command, context, iterations),
                        "declared_files": [str(path) for path in backend.declared_files],
                        "preparation": preparation,
                        "machine_state_before": before,
                        "machine_state_after": after,
                        "stdout_log": str(stdout_path),
                        "stderr_log": str(stderr_path),
                        "stderr_sha256": sha256_file(stderr_path),
                        "transcript": transcript_meta,
                        "measurement": stripped,
                    }
                    append_jsonl(raw_path, record)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        repo = args.repo.resolve()
        results_root = args.results_root.resolve()
        if results_root.exists() and any(results_root.iterdir()):
            raise EvidenceError(f"results root must be empty: {results_root}")
        results_root.mkdir(parents=True, exist_ok=True)
        harness_commit, harness_branch, harness_hashes = verify_harness(repo)
        backends, manifest_sha256 = load_backends(args.manifest.resolve())
        host = host_environment()
        base_identity = {
            backend.backend: identity_fields(
                backend,
                harness_commit,
                harness_branch,
                harness_hashes,
                manifest_sha256,
                host,
            )
            for backend in backends
        }
        backend_registry = [backend.backend for backend in backends]
        for value in base_identity.values():
            value.update(
                {
                    "backend_registry": backend_registry,
                    "availability_context_registry": list(AVAILABILITY_CONTEXTS),
                    "cache_stance_registry": list(CACHE_STANCES),
                    "planned_samples_per_row": args.samples,
                }
            )
        baseline = parity_phase(
            backends,
            results_root,
            args.parity_iterations,
            args.timeout_seconds,
            base_identity,
        )
        measurement_phase(
            backends,
            results_root,
            baseline,
            args.samples,
            args.iterations,
            args.timeout_seconds,
            base_identity,
        )
        print(results_root / "raw" / "measurements.jsonl")
        return 0
    except (EvidenceError, OSError, subprocess.SubprocessError, json.JSONDecodeError) as error:
        print(f"S83-AV1 evidence failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
