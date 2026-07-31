#!/usr/bin/env python3
"""Summarize S83-AV1 raw evidence without selecting a candidate."""

from __future__ import annotations

import argparse
import json
import re
import statistics
import sys
from collections import defaultdict
from pathlib import Path
from typing import Any, Iterable, Sequence


RAW_SCHEMA = "hbk-s83-av1-raw/v1"
SUMMARY_SCHEMA = "hbk-s83-av1-summary/v1"
REPORT_SCHEMA = "hbk-s83-av1-benchmark/v1"
WORKLOAD_VERSION = "s83-av1-filtered-global-method-enumeration/v1"
ORCHESTRATION_VERSION = "hbk-s83-av1-orchestration/v1"
DATASET = "shcntx_ru-8.3.27.1859-schema16-extraction11"
PLATFORM_VERSION = "8.3.27.1859"
PROVIDER_SCHEMA_VERSION = 16
EXTRACTION_SCHEMA_VERSION = 11
HBK_SHA256 = "5bdf0b3ed89932572c012faddc4d05ebfa2986595cf2849b6eb6e5e65a9a4d48"
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
DEFAULT_EXPECTED_SAMPLES = 9
NOISE_LIMIT = 0.05
FORBIDDEN_METADATA_KEY = re.compile(r"module_?context_?kind", re.IGNORECASE)
MEASUREMENT_KEYS = frozenset(
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

METRICS: dict[str, tuple[str, ...]] = {
    "entry_to_ready_ns": ("measurement", "timings", "entry_to_ready_ns"),
    "first_enumeration_ns": (
        "measurement",
        "timings",
        "first_enumeration",
        "elapsed_ns",
    ),
    "first_enumeration_ns_per_object": (
        "measurement",
        "timings",
        "first_enumeration",
        "ns_per_object",
    ),
    "first_enumeration_minor_faults": (
        "measurement",
        "timings",
        "first_enumeration",
        "faults",
        "minor",
    ),
    "first_enumeration_major_faults": (
        "measurement",
        "timings",
        "first_enumeration",
        "faults",
        "major",
    ),
    "warm_workload_total_ns": ("measurement", "timings", "workload", "elapsed_ns"),
    "warm_workload_ns_per_enumeration": (
        "measurement",
        "timings",
        "workload",
        "average_ns",
    ),
    "warm_workload_ns_per_object": (
        "measurement",
        "timings",
        "workload",
        "ns_per_object",
    ),
    "warm_workload_minor_faults": (
        "measurement",
        "timings",
        "workload",
        "faults",
        "minor",
    ),
    "warm_workload_major_faults": (
        "measurement",
        "timings",
        "workload",
        "faults",
        "major",
    ),
    "first_enumeration_allocation_calls": (
        "measurement",
        "allocations",
        "first_enumeration",
        "allocation_calls",
    ),
    "first_enumeration_allocated_bytes": (
        "measurement",
        "allocations",
        "first_enumeration",
        "allocated_bytes",
    ),
    "warm_workload_allocation_calls": (
        "measurement",
        "allocations",
        "workload",
        "allocation_calls",
    ),
    "warm_workload_allocated_bytes": (
        "measurement",
        "allocations",
        "workload",
        "allocated_bytes",
    ),
    "final_live_bytes": (
        "measurement",
        "allocations",
        "final_snapshot",
        "current_live_bytes",
    ),
    "final_peak_live_bytes": (
        "measurement",
        "allocations",
        "final_snapshot",
        "peak_live_bytes",
    ),
}


class SummaryError(RuntimeError):
    """Raw evidence is incomplete or internally inconsistent."""


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("raw_jsonl", nargs="+", type=Path)
    parser.add_argument("--expected-samples", type=positive_int, default=DEFAULT_EXPECTED_SAMPLES)
    parser.add_argument("--json", dest="json_path", type=Path, required=True)
    parser.add_argument("--markdown", dest="markdown_path", type=Path, required=True)
    return parser.parse_args(argv)


def positive_int(value: str) -> int:
    parsed = int(value)
    if parsed <= 0:
        raise argparse.ArgumentTypeError("value must be greater than zero")
    return parsed


def read_jsonl(paths: Iterable[Path]) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    for path in paths:
        with path.open(encoding="utf-8") as stream:
            for line_number, line in enumerate(stream, 1):
                if not line.strip():
                    continue
                try:
                    value = json.loads(line)
                except json.JSONDecodeError as error:
                    raise SummaryError(f"{path}:{line_number}: invalid JSON") from error
                if not isinstance(value, dict):
                    raise SummaryError(f"{path}:{line_number}: expected an object")
                value["_source_path"] = str(path)
                value["_source_line"] = line_number
                records.append(value)
    if not records:
        raise SummaryError("no raw records")
    return records


def nested(value: Any, path: Sequence[str]) -> Any:
    for key in path:
        if not isinstance(value, dict) or key not in value:
            return None
        value = value[key]
    return value


def number(value: Any, path: str) -> int | float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise SummaryError(f"{path}: expected a number, got {value!r}")
    return value


def forbidden_metadata_paths(value: Any, prefix: str = "") -> list[str]:
    matches: list[str] = []
    if isinstance(value, dict):
        for key, nested_value in value.items():
            path = f"{prefix}.{key}" if prefix else str(key)
            if (
                key != "module_context_filter_used"
                and FORBIDDEN_METADATA_KEY.search(str(key))
            ):
                matches.append(path)
            matches.extend(forbidden_metadata_paths(nested_value, path))
    elif isinstance(value, list):
        for index, nested_value in enumerate(value):
            matches.extend(forbidden_metadata_paths(nested_value, f"{prefix}[{index}]"))
    return matches


def validate_artifact_identity(value: Any, source: str, field: str) -> None:
    if not isinstance(value, dict):
        raise SummaryError(f"{source}: {field} must be an object")
    path = value.get("path")
    bytes_value = value.get("bytes")
    sha256 = value.get("sha256")
    if not isinstance(path, str) or not path:
        raise SummaryError(f"{source}: {field}.path must be a non-empty string")
    if isinstance(bytes_value, bool) or not isinstance(bytes_value, int) or bytes_value <= 0:
        raise SummaryError(f"{source}: {field}.bytes must be a positive integer")
    if not isinstance(sha256, str) or re.fullmatch(r"[0-9a-f]{64}", sha256) is None:
        raise SummaryError(f"{source}: {field}.sha256 must be a lowercase SHA-256")


def require_exact_keys(
    value: dict[str, Any], expected: frozenset[str], source: str, field: str
) -> None:
    actual = set(value)
    if actual != expected:
        raise SummaryError(
            f"{source}: {field} schema keys differ: "
            f"missing={sorted(expected - actual)}, unexpected={sorted(actual - expected)}"
        )


def normalized_artifact(
    value: Any, source: str, field: str, worktree: Path
) -> tuple[str, int, str]:
    validate_artifact_identity(value, source, field)
    path = Path(value["path"])
    resolved = path.resolve() if path.is_absolute() else (worktree / path).resolve()
    return (str(resolved), value["bytes"], value["sha256"])


def require_object(value: Any, source: str, field: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise SummaryError(f"{source}: {field} must be an object")
    return value


def require_integer(value: Any, source: str, field: str, *, positive: bool = False) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        raise SummaryError(f"{source}: {field} must be an integer")
    if positive and value <= 0:
        raise SummaryError(f"{source}: {field} must be greater than zero")
    return value


def validate_faults(value: Any, source: str, field: str) -> None:
    faults = require_object(value, source, field)
    require_exact_keys(faults, FAULT_KEYS, source, field)
    require_integer(faults.get("minor"), source, f"{field}.minor")
    require_integer(faults.get("major"), source, f"{field}.major")


def validate_phase(value: Any, source: str, field: str) -> None:
    phase = require_object(value, source, field)
    require_exact_keys(phase, PHASE_KEYS, source, field)
    require_integer(phase.get("elapsed_ns"), source, f"{field}.elapsed_ns", positive=True)
    validate_faults(phase.get("faults"), source, f"{field}.faults")


def validate_enumeration_phase(
    value: Any, source: str, field: str, returned_objects: int
) -> None:
    phase = require_object(value, source, field)
    require_exact_keys(phase, ENUMERATION_PHASE_KEYS, source, field)
    require_integer(phase.get("elapsed_ns"), source, f"{field}.elapsed_ns", positive=True)
    ns_per_object = phase.get("ns_per_object")
    if ns_per_object is not None:
        require_integer(ns_per_object, source, f"{field}.ns_per_object")
    validate_faults(phase.get("faults"), source, f"{field}.faults")
    if phase.get("returned_objects") != returned_objects:
        raise SummaryError(f"{source}: {field}.returned_objects differs from measurement counts")
    require_integer(phase.get("checksum"), source, f"{field}.checksum")


def validate_workload(
    value: Any, source: str, field: str, iterations: int, returned_objects: int
) -> None:
    workload = require_object(value, source, field)
    require_exact_keys(workload, WORKLOAD_KEYS, source, field)
    require_integer(workload.get("elapsed_ns"), source, f"{field}.elapsed_ns", positive=True)
    require_integer(workload.get("average_ns"), source, f"{field}.average_ns", positive=True)
    ns_per_object = workload.get("ns_per_object")
    if ns_per_object is not None:
        require_integer(ns_per_object, source, f"{field}.ns_per_object")
    validate_faults(workload.get("faults"), source, f"{field}.faults")
    if workload.get("iterations") != iterations:
        raise SummaryError(f"{source}: {field}.iterations differs from raw metadata")
    if workload.get("returned_total") != iterations * returned_objects:
        raise SummaryError(f"{source}: {field}.returned_total is inconsistent")
    require_integer(workload.get("checksum"), source, f"{field}.checksum")


def validate_allocation_delta(value: Any, source: str, field: str) -> None:
    delta = require_object(value, source, field)
    require_exact_keys(delta, ALLOCATION_DELTA_KEYS, source, field)
    for key in ALLOCATION_DELTA_KEYS:
        require_integer(delta.get(key), source, f"{field}.{key}")


def validate_allocation_snapshot(value: Any, source: str, field: str) -> None:
    snapshot = require_object(value, source, field)
    require_exact_keys(snapshot, ALLOCATION_SNAPSHOT_KEYS, source, field)
    for key in ALLOCATION_SNAPSHOT_KEYS:
        require_integer(snapshot.get(key), source, f"{field}.{key}")


def validate_runtime_artifact_binding(
    record: dict[str, Any], measurement: dict[str, Any], source: str
) -> None:
    worktree_value = record.get("worktree")
    if not isinstance(worktree_value, str) or not Path(worktree_value).is_absolute():
        raise SummaryError(f"{source}: worktree must be an absolute path")
    worktree = Path(worktree_value)
    declared_values = record.get("declared_file_artifacts")
    if not isinstance(declared_values, list) or not declared_values:
        raise SummaryError(f"{source}: missing declared_file_artifacts")
    declared = {
        normalized_artifact(value, source, f"declared_file_artifacts[{index}]", worktree)
        for index, value in enumerate(declared_values)
    }
    if len(declared) != len(declared_values):
        raise SummaryError(f"{source}: duplicate declared_file_artifacts")

    backend = record.get("backend")
    index = measurement.get("index")
    cache = measurement.get("cache")
    cache_status = measurement.get("cache_status")
    if backend == "S83-H0":
        if not isinstance(index, dict) or cache is not None or cache_status is not None:
            raise SummaryError(f"{source}: H0 runtime artifact contract mismatch")
        runtime_values = [index]
    elif backend == "S83-C0":
        if not isinstance(index, dict) or not isinstance(cache, dict) or cache_status != "loaded":
            raise SummaryError(f"{source}: C0 runtime artifact contract mismatch")
        runtime_values = [index, cache]
    elif isinstance(cache, dict) and cache_status == "loaded":
        runtime_values = [cache]
    elif cache is None and isinstance(index, dict) and cache_status == "mapped-checked":
        runtime_values = [index]
    else:
        raise SummaryError(f"{source}: candidate runtime artifact contract mismatch")
    runtime = {
        normalized_artifact(value, source, f"runtime_artifacts[{index}]", worktree)
        for index, value in enumerate(runtime_values)
    }
    if declared != runtime:
        raise SummaryError(f"{source}: declared_file_artifacts differ from runtime artifacts")

    declared_paths = {path for path, _bytes, _sha256 in declared}
    raw_declared_files = record.get("declared_files")
    if (
        not isinstance(raw_declared_files, list)
        or any(not isinstance(path, str) for path in raw_declared_files)
        or len(raw_declared_files) != len(declared_paths)
        or set(raw_declared_files) != declared_paths
    ):
        raise SummaryError(f"{source}: declared_files differ from artifact provenance")
    preparation = record.get("preparation")
    prepared_files = preparation.get("declared_files") if isinstance(preparation, dict) else None
    if (
        not isinstance(prepared_files, list)
        or any(not isinstance(path, str) for path in prepared_files)
        or len(prepared_files) != len(declared_paths)
        or set(prepared_files) != declared_paths
    ):
        raise SummaryError(f"{source}: prepared files differ from runtime artifacts")


def metric(values: Sequence[int | float]) -> dict[str, Any]:
    median = statistics.median(values)
    mad = statistics.median(abs(value - median) for value in values)
    mad_ratio = None if median == 0 else float(mad / abs(median))
    return {
        "samples": len(values),
        "median": median,
        "mad": mad,
        "mad_ratio": mad_ratio,
        "noise_status": (
            "zero-baseline"
            if median == 0
            else "noisy"
            if mad_ratio is not None and mad_ratio > NOISE_LIMIT
            else "stable"
        ),
        "values": list(values),
    }


def identity(record: dict[str, Any]) -> str:
    fields = {
        "dataset": record.get("dataset"),
        "platform_version": record.get("platform_version"),
        "provider_schema_version": record.get("provider_schema_version"),
        "extraction_schema_version": record.get("extraction_schema_version"),
        "hbk_sha256": record.get("hbk_sha256"),
        "provider_sha256": record.get("provider_sha256"),
        "harness_commit": record.get("harness_commit"),
        "harness_file_sha256": record.get("harness_file_sha256"),
        "manifest_sha256": record.get("manifest_sha256"),
        "host": record.get("host"),
        "orchestration_version": record.get("orchestration_version"),
        "iterations": record.get("iterations"),
        "backend_registry": record.get("backend_registry"),
        "availability_context_registry": record.get("availability_context_registry"),
        "cache_stance_registry": record.get("cache_stance_registry"),
        "planned_samples_per_row": record.get("planned_samples_per_row"),
    }
    return json.dumps(fields, ensure_ascii=False, sort_keys=True, separators=(",", ":"))


def validate_record(record: dict[str, Any]) -> None:
    source = f"{record.get('_source_path')}:{record.get('_source_line')}"
    forbidden = forbidden_metadata_paths(record)
    if forbidden:
        raise SummaryError(f"{source}: ModuleContextKind metadata is forbidden: {forbidden}")
    expected = {
        "schema": RAW_SCHEMA,
        "dataset": DATASET,
        "platform_version": PLATFORM_VERSION,
        "provider_schema_version": PROVIDER_SCHEMA_VERSION,
        "extraction_schema_version": EXTRACTION_SCHEMA_VERSION,
        "hbk_sha256": HBK_SHA256,
        "provider_sha256": PROVIDER_SHA256,
        "baseline_role": "h0",
        "module_context_filter_used": False,
        "empty_availability_rule": "universal",
        "orchestration_version": ORCHESTRATION_VERSION,
        "status": "ok",
    }
    for key, expected_value in expected.items():
        if record.get(key) != expected_value:
            raise SummaryError(
                f"{source}: {key} expected {expected_value!r}, got {record.get(key)!r}"
            )
    backend = record.get("backend")
    role = record.get("decision_role")
    expected_role = (
        "baseline" if backend == "S83-H0" else "control" if backend == "S83-C0" else "candidate"
    )
    if role != expected_role:
        raise SummaryError(f"{source}: {backend} must have decision_role={expected_role}")
    if record.get("availability_context") not in AVAILABILITY_CONTEXTS:
        raise SummaryError(f"{source}: unknown availability context")
    if record.get("cache_stance") not in CACHE_STANCES:
        raise SummaryError(f"{source}: unknown cache stance")
    sample = record.get("sample")
    if isinstance(sample, bool) or not isinstance(sample, int) or sample <= 0:
        raise SummaryError(f"{source}: invalid sample id {sample!r}")
    transcript = record.get("transcript")
    if not isinstance(transcript, dict) or transcript.get("parity_status") != "pass":
        raise SummaryError(f"{source}: transcript parity mismatch or missing")
    if transcript.get("sha256") != transcript.get("baseline_sha256"):
        raise SummaryError(f"{source}: transcript digest differs from H0")
    if not isinstance(transcript.get("sha256"), str) or len(transcript["sha256"]) != 64:
        raise SummaryError(f"{source}: invalid transcript digest")
    for field in ("bytes", "item_count"):
        value = transcript.get(field)
        if isinstance(value, bool) or not isinstance(value, int) or value <= 0:
            raise SummaryError(f"{source}: transcript.{field} must be positive")
    if not isinstance(record.get("measurement"), dict):
        raise SummaryError(f"{source}: missing stripped measurement")
    if "transcript" in record["measurement"]:
        raise SummaryError(f"{source}: raw measurement retained the large transcript")
    measurement = record["measurement"]
    require_exact_keys(measurement, MEASUREMENT_KEYS, source, "measurement")
    measurement_expected = {
        "schema_version": REPORT_SCHEMA,
        "workload_version": WORKLOAD_VERSION,
        "backend": backend,
        "decision_role": role,
        "baseline_role": "h0",
        "module_context_filter_used": False,
        "empty_availability_rule": "universal",
        "availability_context": record.get("availability_context"),
        "iterations": record.get("iterations"),
    }
    for key, expected_value in measurement_expected.items():
        if measurement.get(key) != expected_value:
            raise SummaryError(f"{source}: measurement.{key} differs from raw metadata")
    counts = measurement.get("counts")
    timings = measurement.get("timings")
    if not isinstance(counts, dict) or not isinstance(timings, dict):
        raise SummaryError(f"{source}: measurement counts/timings must be objects")
    require_exact_keys(counts, COUNT_KEYS, source, "measurement.counts")
    require_exact_keys(timings, TIMING_KEYS, source, "measurement.timings")
    validate_runtime_artifact_binding(record, measurement, source)
    returned_objects = nested(measurement, ("counts", "returned_objects"))
    if transcript.get("item_count") != returned_objects:
        raise SummaryError(f"{source}: transcript item count differs from measurement")
    iterations = record.get("iterations")
    if isinstance(iterations, bool) or not isinstance(iterations, int) or iterations <= 0:
        raise SummaryError(f"{source}: iterations must be positive")
    validate_phase(timings.get("open"), source, "measurement.timings.open")
    validate_enumeration_phase(
        timings.get("first_enumeration"),
        source,
        "measurement.timings.first_enumeration",
        returned_objects,
    )
    validate_enumeration_phase(
        timings.get("warmup"),
        source,
        "measurement.timings.warmup",
        returned_objects,
    )
    validate_workload(
        timings.get("workload"),
        source,
        "measurement.timings.workload",
        iterations,
        returned_objects,
    )
    if nested(measurement, ("allocations", "enabled")) is not True:
        raise SummaryError(f"{source}: allocation instrumentation is not enabled")
    allocations = require_object(measurement.get("allocations"), source, "measurement.allocations")
    require_exact_keys(allocations, ALLOCATIONS_KEYS, source, "measurement.allocations")
    for phase in ("entry_to_ready", "first_enumeration", "warmup", "workload"):
        validate_allocation_delta(
            allocations.get(phase), source, f"measurement.allocations.{phase}"
        )
    validate_allocation_snapshot(
        allocations.get("final_snapshot"), source, "measurement.allocations.final_snapshot"
    )
    for key in ("candidate_commit", "candidate_branch", "harness_commit"):
        value = record.get(key)
        if not isinstance(value, str) or not value:
            raise SummaryError(f"{source}: missing {key}")
    for key in ("command", "command_template"):
        value = record.get(key)
        if not isinstance(value, list) or not value or any(not isinstance(item, str) for item in value):
            raise SummaryError(f"{source}: missing {key} argv metadata")
    if not isinstance(record.get("host"), dict):
        raise SummaryError(f"{source}: missing host metadata")
    if record.get("availability_context_registry") != list(AVAILABILITY_CONTEXTS):
        raise SummaryError(f"{source}: wrong availability context registry")
    if record.get("cache_stance_registry") != list(CACHE_STANCES):
        raise SummaryError(f"{source}: wrong cache stance registry")
    backend_registry = record.get("backend_registry")
    if (
        not isinstance(backend_registry, list)
        or any(not isinstance(value, str) for value in backend_registry)
        or len(backend_registry) != len(set(backend_registry))
        or backend not in backend_registry
    ):
        raise SummaryError(f"{source}: invalid backend registry")
    for key in ("machine_state_before", "machine_state_after", "preparation"):
        if not isinstance(record.get(key), dict):
            raise SummaryError(f"{source}: missing {key} metadata")


def build_summary(records: list[dict[str, Any]], expected_samples: int) -> dict[str, Any]:
    for record in records:
        validate_record(record)
    identities = {identity(record) for record in records}
    if len(identities) != 1:
        raise SummaryError(f"mixed corpus/harness/manifest identities: {identities}")
    corpus = json.loads(next(iter(identities)))
    if corpus.get("planned_samples_per_row") != expected_samples:
        raise SummaryError(
            "expected sample count differs from the orchestration plan: "
            f"{expected_samples} != {corpus.get('planned_samples_per_row')}"
        )

    backend_metadata: dict[str, tuple[Any, ...]] = {}
    backend_order = list(corpus["backend_registry"])
    groups: dict[tuple[str, str, str], list[dict[str, Any]]] = defaultdict(list)
    seen_samples: set[tuple[str, str, str, int]] = set()
    for record in records:
        backend = str(record["backend"])
        metadata = (
            record.get("decision_role"),
            record.get("candidate_commit"),
            record.get("candidate_branch"),
            record.get("harness_commit"),
            json.dumps(
                record.get("declared_file_artifacts"),
                ensure_ascii=False,
                sort_keys=True,
                separators=(",", ":"),
            ),
        )
        previous = backend_metadata.setdefault(backend, metadata)
        if previous != metadata:
            raise SummaryError(f"{backend}: mixed branch/commit/role metadata")
        if backend not in backend_order:
            raise SummaryError(f"raw backend is absent from registry: {backend}")
        key = (backend, str(record["availability_context"]), str(record["cache_stance"]))
        sample_key = (*key, int(record["sample"]))
        if sample_key in seen_samples:
            raise SummaryError(f"duplicate sample: {sample_key}")
        seen_samples.add(sample_key)
        groups[key].append(record)

    if "S83-H0" not in backend_metadata or "S83-C0" not in backend_metadata:
        raise SummaryError("raw evidence must contain S83-H0 and S83-C0")
    expected_ids = list(range(1, expected_samples + 1))
    aggregates: dict[tuple[str, str, str], dict[str, Any]] = {}
    for backend in backend_order:
        for context in AVAILABILITY_CONTEXTS:
            for stance in CACHE_STANCES:
                key = (backend, context, stance)
                group = groups.get(key, [])
                sample_ids = sorted(record["sample"] for record in group)
                if sample_ids != expected_ids:
                    raise SummaryError(
                        f"{backend}/{context}/{stance}: expected samples {expected_ids}, "
                        f"got {sample_ids}"
                    )
                transcript_sha = {record["transcript"]["sha256"] for record in group}
                baseline_sha = {record["transcript"]["baseline_sha256"] for record in group}
                if len(transcript_sha) != 1 or transcript_sha != baseline_sha:
                    raise SummaryError(f"{backend}/{context}/{stance}: parity drift")
                metrics: dict[str, Any] = {}
                for name, path in METRICS.items():
                    values = [
                        number(
                            nested(record, path),
                            f"{backend}/{context}/{stance}/{record['sample']}:{'.'.join(path)}",
                        )
                        for record in sorted(group, key=lambda item: item["sample"])
                    ]
                    metrics[name] = metric(values)
                counts = group[0]["measurement"].get("counts")
                if any(record["measurement"].get("counts") != counts for record in group):
                    raise SummaryError(f"{backend}/{context}/{stance}: count drift")
                aggregates[key] = {
                    "backend": backend,
                    "decision_role": backend_metadata[backend][0],
                    "candidate_commit": backend_metadata[backend][1],
                    "candidate_branch": backend_metadata[backend][2],
                    "availability_context": context,
                    "cache_stance": stance,
                    "sample_status": "complete",
                    "parity_status": "pass",
                    "transcript_sha256": next(iter(transcript_sha)),
                    "counts": counts,
                    "metrics": metrics,
                }

    for backend in backend_order:
        for context in AVAILABILITY_CONTEXTS:
            warm = aggregates[(backend, context, "warm")]
            cold = aggregates[(backend, context, "cold-best-effort")]
            if warm["transcript_sha256"] != cold["transcript_sha256"]:
                raise SummaryError(f"{backend}/{context}: transcript differs by cache stance")
            if warm["counts"] != cold["counts"]:
                raise SummaryError(f"{backend}/{context}: counts differ by cache stance")

    rows: list[dict[str, Any]] = []
    for backend in backend_order:
        for context in AVAILABILITY_CONTEXTS:
            for stance in CACHE_STANCES:
                aggregate = aggregates[(backend, context, stance)]
                baseline = aggregates[("S83-H0", context, stance)]
                if aggregate["transcript_sha256"] != baseline["transcript_sha256"]:
                    raise SummaryError(
                        f"{backend}/{context}/{stance}: transcript differs from S83-H0"
                    )
                if aggregate["counts"] != baseline["counts"]:
                    raise SummaryError(
                        f"{backend}/{context}/{stance}: enumeration counts differ from S83-H0"
                    )
                for name, value in aggregate["metrics"].items():
                    denominator = baseline["metrics"][name]["median"]
                    value["ratio_to_h0"] = (
                        None if denominator == 0 else float(value["median"] / denominator)
                    )
                rows.append(aggregate)

    return {
        "schema": SUMMARY_SCHEMA,
        "workload_version": WORKLOAD_VERSION,
        "decision_policy": {
            "baseline": "S83-H0",
            "control": "S83-C0",
            "decision_state": "user-decision-required",
            "automatic_candidate_choice": False,
        },
        "identity": corpus,
        "expected_samples_per_row": expected_samples,
        "availability_contexts": list(AVAILABILITY_CONTEXTS),
        "cache_stances": list(CACHE_STANCES),
        "rows": rows,
    }


def markdown(summary: dict[str, Any]) -> str:
    lines = [
        "# S83-AV1: enumeration с фильтром по AvailabilityContext",
        "",
        "Baseline — `S83-H0` (SQLite-to-owned). `S83-C0` приведён только как контроль. "
        "Таблица не выбирает кандидата; решение остаётся за пользователем.",
        "",
        "| Вариант | Роль | AvailabilityContext | Cache | Объектов | First, median ns | "
        "First/H0 | Warm enum, median ns | Warm/H0 | MAD first | MAD warm | Parity |",
        "| --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |",
    ]
    for row in summary["rows"]:
        first = row["metrics"]["first_enumeration_ns"]
        warm = row["metrics"]["warm_workload_ns_per_enumeration"]
        lines.append(
            "| {backend} | {role} | `{context}` | {stance} | {objects} | {first_median} | "
            "{first_ratio} | {warm_median} | {warm_ratio} | {first_mad} | {warm_mad} | {parity} |".format(
                backend=row["backend"],
                role=row["decision_role"],
                context=row["availability_context"],
                stance=row["cache_stance"],
                objects=row["counts"]["returned_objects"],
                first_median=first["median"],
                first_ratio=format_ratio(first["ratio_to_h0"]),
                warm_median=warm["median"],
                warm_ratio=format_ratio(warm["ratio_to_h0"]),
                first_mad=first["mad"],
                warm_mad=warm["mad"],
                parity=row["parity_status"],
            )
        )
    lines.extend(
        [
            "",
            "Полная JSON-сводка дополнительно содержит ns/объект, page faults, allocation calls/bytes, "
            "median, MAD и отношение каждого числового показателя к H0.",
            "",
        ]
    )
    return "\n".join(lines)


def format_ratio(value: Any) -> str:
    return "n/a" if value is None else f"{value:.3f}×"


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        summary = build_summary(read_jsonl(args.raw_jsonl), args.expected_samples)
        args.json_path.parent.mkdir(parents=True, exist_ok=True)
        args.markdown_path.parent.mkdir(parents=True, exist_ok=True)
        args.json_path.write_text(
            json.dumps(summary, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
        )
        args.markdown_path.write_text(markdown(summary), encoding="utf-8")
        return 0
    except (OSError, SummaryError, json.JSONDecodeError) as error:
        print(f"S83-AV1 summary failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
