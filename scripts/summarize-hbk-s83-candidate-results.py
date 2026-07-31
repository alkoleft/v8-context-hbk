#!/usr/bin/env python3
"""Summarize completed S83 candidate evidence without making a selection."""

from __future__ import annotations

import argparse
import datetime as dt
import json
import statistics
import subprocess
import sys
from collections import defaultdict
from pathlib import Path
from typing import Any, Iterable


S83_HARNESS_COMMIT = "28f29b5a262db362b6b58c8109e6df6c2afbbc44"
S83_DATASET = "shcntx_ru-8.3.27.1859-schema16-extraction11"
S83_PLATFORM = "8.3.27.1859"
S83_SQLITE_SHA256 = "55c2e09971712a13a49cbcf5889f203d7a9dfcec22aa0d333247ae722f6f0fab"
S83_HBK_SHA256 = "5bdf0b3ed89932572c012faddc4d05ebfa2986595cf2849b6eb6e5e65a9a4d48"
S83_CONTENT_SHA256 = "5f66d20509877ac29a83ede2d5178368ed3fd78d7dab0ffbc12df506acc3b1fd"
S83_LOOKUP_SHA256 = "9b17c7100cd368fe0880e679d66ab8eb7d8505ee617d9fc80b1a9a9d8aa5c5c8"
S83_SEMANTIC_SHA256 = "1fe7f166caad8e8573b809a97f7104caf85301370f1d34017376bc82ee893a29"
S83_SEMANTIC_RECORDS = 742_872
S83_SEMANTIC_BYTES = 769_824_709
S83_REVERSE_DICTIONARY_HIT_NS = 458
S83_REVERSE_DICTIONARY_MISS_NS = 24_048

EXPECTED_COUNTS = {
    "runtime:warm": 9,
    "runtime:cold-best-effort": 9,
    "production:warm": 9,
    "runtime-allocation:warm": 3,
    "producer-allocation:warm": 3,
    "four-reader:warm": 3,
}

REGISTRY_ORDER = ("s83-f0", "s83-a0", "s83-l1", "s83-i1", "s83-d1", "s83-p1", "s83-r1")

GATES = {
    "warm_ready_ns": ("runtime", "warm", "ready_ns", 33_991_352),
    "cold_ready_ns": ("runtime", "cold-best-effort", "ready_ns", 59_020_968),
    "runtime_allocation_calls": (
        "runtime_allocation",
        "warm",
        "allocation_calls",
        68_018,
    ),
    "runtime_allocated_bytes": (
        "runtime_allocation",
        "warm",
        "allocated_bytes",
        14_471_464,
    ),
    "warm_peak_rss_kib": ("runtime", "warm", "peak_rss_kib", 29_593),
    "cold_peak_rss_kib": (
        "runtime",
        "cold-best-effort",
        "peak_rss_kib",
        29_593,
    ),
    "warm_workload_pss_kib": ("runtime", "warm", "workload_pss_kib", 17_712),
    "warm_workload_private_kib": (
        "runtime",
        "warm",
        "workload_private_kib",
        17_696,
    ),
    "cold_workload_pss_kib": (
        "runtime",
        "cold-best-effort",
        "workload_pss_kib",
        17_681,
    ),
    "cold_workload_private_kib": (
        "runtime",
        "cold-best-effort",
        "workload_private_kib",
        17_664,
    ),
    "four_reader_pss_kib": ("four_reader", "warm", "pss_kib", 64_913),
    "warm_first_lookup_ns": ("runtime", "warm", "first_lookup_ns", 25_000),
    "cold_first_lookup_ns": (
        "runtime",
        "cold-best-effort",
        "first_lookup_ns",
        25_000,
    ),
    "warm_anchor_resolution_ns": (
        "runtime",
        "warm",
        "anchor_resolution_ns",
        25_000,
    ),
    "cold_anchor_resolution_ns": (
        "runtime",
        "cold-best-effort",
        "anchor_resolution_ns",
        25_000,
    ),
    "warm_workload_ns": ("runtime", "warm", "workload_ns", 2_381_766_522),
    "cold_workload_ns": (
        "runtime",
        "cold-best-effort",
        "workload_ns",
        2_405_076_745,
    ),
    "open_minor_faults": ("runtime_max", "all", "open_minor_faults", 9_525),
    "open_major_faults": ("runtime_max", "all", "open_major_faults", 0),
    "cold_file_resident_growth_bytes": (
        "runtime",
        "cold-best-effort",
        "file_resident_growth_bytes",
        14_074_880,
    ),
    "artifact_bytes": ("production", "warm", "artifact_bytes", 13_982_571),
    "production_total_ns": ("production", "warm", "total_ns", 803_548_621),
    "production_peak_rss_kib": (
        "production",
        "warm",
        "peak_rss_kib",
        100_975,
    ),
    "producer_allocation_calls": (
        "producer_allocation",
        "warm",
        "allocation_calls",
        1_597_946,
    ),
    "producer_allocated_bytes": (
        "producer_allocation",
        "warm",
        "allocated_bytes",
        229_823_958,
    ),
    "producer_peak_live_bytes": (
        "producer_allocation",
        "warm",
        "peak_live_bytes",
        78_772_998,
    ),
}


class SummaryError(RuntimeError):
    pass


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("raw_jsonl", nargs="+", type=Path)
    parser.add_argument("--harness-commit", default=S83_HARNESS_COMMIT)
    parser.add_argument("--baseline-summary", type=Path, required=True)
    parser.add_argument("--json", dest="json_path", type=Path, required=True)
    parser.add_argument("--markdown", dest="markdown_path", type=Path, required=True)
    parser.add_argument("--storage-parity-raw", action="append", type=Path, default=[])
    parser.add_argument("--semantic-parity-raw", action="append", type=Path, default=[])
    parser.add_argument("--storage-parity-status", action="append", default=[])
    parser.add_argument("--semantic-parity-status", action="append", default=[])
    return parser.parse_args()


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    records = []
    with path.open(encoding="utf-8") as source:
        for line_number, line in enumerate(source, 1):
            if not line.strip():
                continue
            value = json.loads(line)
            if not isinstance(value, dict):
                raise SummaryError(f"{path}:{line_number}: expected object")
            value["_source_path"] = str(path)
            records.append(value)
    return records


def nested(value: Any, *path: str) -> Any:
    for key in path:
        if not isinstance(value, dict) or key not in value:
            return None
        value = value[key]
    return value


def number(value: Any) -> int | float | None:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        return None
    return value


def metric(values: Iterable[int | float]) -> dict[str, Any] | None:
    values = list(values)
    if not values:
        return None
    median = statistics.median(values)
    mad = statistics.median(abs(value - median) for value in values)
    ratio = None if median == 0 else float(mad / median)
    return {
        "samples": len(values),
        "median": median,
        "mad": mad,
        "mad_ratio": ratio,
        "noisy": ratio is not None and ratio > 0.05,
        "values": values,
    }


def require_metric(
    values: Iterable[int | float], name: str = "unnamed metric"
) -> dict[str, Any]:
    result = metric(values)
    if result is None:
        raise SummaryError(f"missing metric {name}")
    return result


def require_metric_exact(
    values: Iterable[int | float], expected: int, name: str
) -> dict[str, Any]:
    values = list(values)
    if len(values) != expected:
        raise SummaryError(f"{name}: expected {expected} values, got {len(values)}")
    return require_metric(values, name)


def require_sample_ids(
    records: list[dict[str, Any]], expected: int, name: str
) -> None:
    sample_ids = [record.get("sample") for record in records]
    expected_ids = list(range(1, expected + 1))
    if (
        any(
            isinstance(sample_id, bool) or not isinstance(sample_id, int)
            for sample_id in sample_ids
        )
        or sorted(sample_ids) != expected_ids
    ):
        raise SummaryError(
            f"{name}: expected unique sample ids {expected_ids}, got {sample_ids}"
        )


def candidate_key(record: dict[str, Any]) -> tuple[str, str, str]:
    return (
        str(record.get("backend", "")),
        str(record.get("candidate_branch", "")),
        str(record.get("candidate_commit", "")),
    )


def runtime_backend(backend: str) -> str:
    return backend[:-8] if backend.endswith("-produce") else backend


def parity_backend(backend: str) -> str:
    lowered = backend.lower()
    for registry_backend in REGISTRY_ORDER:
        if (
            lowered == registry_backend
            or lowered.startswith(f"{registry_backend}-")
            or lowered.startswith(f"{registry_backend}_")
        ):
            return registry_backend
    return backend


def registry_index(backend: str) -> int:
    try:
        return REGISTRY_ORDER.index(backend)
    except ValueError:
        return len(REGISTRY_ORDER)


def scenario(record: dict[str, Any]) -> str:
    raw = record.get("scenario")
    if raw:
        return str(raw)
    if str(record.get("backend", "")).endswith("-produce"):
        return "production"
    return "runtime"


def cache_stance(record: dict[str, Any]) -> str:
    return str(record.get("cache_stance") or "warm")


def corpus_identity(record: dict[str, Any]) -> tuple[Any, ...]:
    return (
        record.get("dataset"),
        record.get("platform_version"),
        record.get("provider_schema_version"),
        record.get("extraction_schema_version"),
        record.get("sqlite_sha256"),
        record.get("hbk_sha256") or record.get("source_hbk_sha256"),
    )


def validate_official_records(
    records: list[dict[str, Any]], harness_commit: str
) -> tuple[Any, ...]:
    expected = (
        S83_DATASET,
        S83_PLATFORM,
        16,
        11,
        S83_SQLITE_SHA256,
        S83_HBK_SHA256,
    )
    identities = set()
    for record in records:
        identity = corpus_identity(record)
        if identity != expected:
            raise SummaryError(
                f"{record.get('_source_path')}: resource record has wrong or "
                f"incomplete S83 corpus identity: {identity}"
            )
        identities.add(identity)
    if len(identities) > 1:
        raise SummaryError(f"mixed corpus identities: {sorted(map(str, identities))}")
    for record in records:
        status = record.get("status")
        if status not in {"ok", "pass"}:
            raise SummaryError(
                f"failed official record in {record.get('_source_path')}: "
                f"{record.get('backend')} {scenario(record)} sample={record.get('sample')} "
                f"status={status}"
            )
        if "harness_commit" in record and record["harness_commit"] != harness_commit:
            raise SummaryError(
                f"wrong harness in {record.get('_source_path')}: {record['harness_commit']}"
            )
    return next(iter(identities), ())


def group_by_identity(groups: dict[str, Any], backend: str, stance: str) -> dict[str, Any]:
    for group in groups.values():
        identity = group.get("identity", {})
        if (
            identity.get("dataset") == S83_DATASET
            and identity.get("backend") == backend
            and identity.get("cache_stance") == stance
            and identity.get("candidate_commit") == S83_HARNESS_COMMIT
        ):
            return group
    raise SummaryError(f"baseline summary missing {backend} {stance}")


def load_baseline_summary(path: Path, harness_commit: str) -> dict[str, Any]:
    summary = json.loads(path.read_text(encoding="utf-8"))
    if summary.get("schema") != "hbk-snapshot-benchmark-summary-v1":
        raise SummaryError(f"unexpected baseline summary schema in {path}")
    if summary.get("harness_commit") != harness_commit:
        raise SummaryError(
            f"wrong baseline summary harness: {summary.get('harness_commit')}"
        )
    groups = summary.get("groups")
    allocations = summary.get("allocation_profiles")
    aggregates = summary.get("aggregate_four_reader")
    if not isinstance(groups, dict) or not isinstance(allocations, dict) or not isinstance(aggregates, dict):
        raise SummaryError("baseline summary is missing groups/allocation/aggregate sections")

    result = {
        "source": str(path),
        "h0": {
            "runtime": {
                stance: group_by_identity(groups, "sql-owned", stance)
                for stance in ("warm", "cold-best-effort")
            },
            "four_reader": group_by_identity(aggregates, "sql-owned", "warm"),
        },
        "c0": {
            "runtime": {
                stance: group_by_identity(groups, "cache-owned", stance)
                for stance in ("warm", "cold-best-effort")
            },
            "production": group_by_identity(groups, "cache-owned-produce", "warm"),
            "runtime_allocation": group_by_identity(allocations, "cache-owned", "warm"),
            "producer_allocation": group_by_identity(
                allocations, "cache-owned-produce", "warm"
            ),
            "four_reader": group_by_identity(aggregates, "cache-owned", "warm"),
        },
    }
    for stance, group in result["c0"]["runtime"].items():
        operations = group.get("operations")
        if not isinstance(operations, dict) or len(operations) != 25:
            raise SummaryError(
                f"baseline C0 {stance} must contain exactly 25 operations"
            )
        for name, operation in operations.items():
            totals = operation.get("observed_totals")
            if not isinstance(totals, list) or len(totals) != 1:
                raise SummaryError(
                    f"baseline C0 {stance} operation {name} has invalid observed totals"
                )
    return result


def production_total_ns(measurement: dict[str, Any]) -> int | float | None:
    return number(nested(measurement, "timings", "total_ns")) or number(
        measurement.get("total_ns")
    )


def production_materialize_ns(measurement: dict[str, Any]) -> int | float | None:
    return number(nested(measurement, "timings", "materialize_ns")) or number(
        measurement.get("materialize_ns")
    )


def production_write_ns(measurement: dict[str, Any]) -> int | float | None:
    direct_write_phases = (
        "temp_create_setup_ns",
        "section_write_ns",
        "header_directory_write_ns",
        "temp_sync_ns",
        "publish_ns",
    )
    direct_values = [
        number(nested(measurement, "timings", phase)) for phase in direct_write_phases
    ]
    direct_write_ns = (
        sum(value for value in direct_values if value is not None)
        if all(value is not None for value in direct_values)
        else None
    )
    return (
        number(nested(measurement, "timings", "write_publish_ns"))
        or number(nested(measurement, "timings", "write_ns"))
        or direct_write_ns
        or number(measurement.get("write_ns"))
    )


def production_serialize_ns(measurement: dict[str, Any]) -> int | float | None:
    return (
        number(nested(measurement, "timings", "serialize_ns"))
        or number(nested(measurement, "timings", "direct_formation_ns"))
        or number(measurement.get("serialize_ns"))
    )


def production_validate_ns(measurement: dict[str, Any]) -> int | float | None:
    return (
        number(nested(measurement, "timings", "in_memory_validation_ns"))
        or number(nested(measurement, "timings", "validate_ns"))
        or number(nested(measurement, "timings", "file_validation_ns"))
        or number(measurement.get("validate_ns"))
    )


def artifact_bytes(measurement: dict[str, Any]) -> int | float | None:
    return (
        number(nested(measurement, "footprint", "artifact_bytes"))
        or number(nested(measurement, "archive_footprint", "artifact_bytes"))
        or number(nested(measurement, "snapshot", "archive_footprint", "artifact_bytes"))
        or number(nested(measurement, "cache", "bytes"))
        or number(nested(measurement, "index", "bytes"))
        or number(measurement.get("artifact_bytes"))
    )


def footprint(measurements: list[dict[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    scalar_paths = {
        "artifact_bytes": (("footprint", "artifact_bytes"), ("archive_footprint", "artifact_bytes"), ("snapshot", "archive_footprint", "artifact_bytes"), ("cache", "bytes"), ("index", "bytes"), ("artifact_bytes",)),
        "section_bytes": (("footprint", "section_bytes"),),
        "dictionary_bytes": (("footprint", "dictionary_bytes"),),
        "index_bytes": (("footprint", "index_bytes"),),
        "mapped_hash_bytes": (("footprint", "mapped_hash_bytes"),),
        "mapped_hash_bucket_bytes": (("footprint", "mapped_hash_bucket_bytes"),),
        "mapped_hash_tables": (("footprint", "mapped_hash_tables"),),
        "mapped_hash_groups": (("footprint", "mapped_hash_groups"),),
        "mapped_hash_buckets": (("footprint", "mapped_hash_buckets"),),
        "mapped_hash_max_probe": (("footprint", "mapped_hash_max_probe"),),
        "record_head_bytes": (("footprint", "record_head_bytes"),),
        "nested_arena_bytes": (("footprint", "nested_arena_bytes"),),
        "dictionary_text_bytes": (("archive_footprint", "dictionary_text_bytes"), ("snapshot", "archive_footprint", "dictionary_text_bytes")),
        "sorted_index_estimated_fixed_bytes": (("archive_footprint", "sorted_index_estimated_fixed_bytes"), ("snapshot", "archive_footprint", "sorted_index_estimated_fixed_bytes")),
    }
    for name, paths in scalar_paths.items():
        values = []
        for measurement in measurements:
            for path in paths:
                value = number(nested(measurement, *path))
                if value is not None:
                    values.append(value)
                    break
        if values:
            result[name] = require_metric(values, name)
    return result


def stable_formation_value(
    measurements: list[dict[str, Any]], name: str
) -> str | bool | None:
    values = [
        nested(measurement, "formation", name)
        for measurement in measurements
        if nested(measurement, "formation", name) is not None
    ]
    if not values:
        return None
    if len(values) != len(measurements):
        raise SummaryError(f"production.formation.{name}: missing sample value")
    first = values[0]
    if any(value != first for value in values[1:]):
        raise SummaryError(f"production.formation.{name}: unstable sample values")
    if not isinstance(first, (str, bool)):
        raise SummaryError(f"production.formation.{name}: expected string or boolean")
    return first


def formation_summary(measurements: list[dict[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for name in (
        "strategy",
        "retains_monolithic_artifact_buffer",
        "retains_at_most_one_completed_section_buffer",
        "write_amplification_scope",
        "peak_working_buffer_scope",
    ):
        value = stable_formation_value(measurements, name)
        if value is not None:
            result[name] = value
    for name in (
        "logical_bytes_written",
        "write_amplification_numerator",
        "write_amplification_denominator",
        "peak_section_buffer_bytes",
        "peak_working_buffer_bytes",
    ):
        values = [
            value
            for measurement in measurements
            if (value := number(nested(measurement, "formation", name))) is not None
        ]
        if values:
            result[name] = require_metric_exact(
                values, len(measurements), f"production.formation.{name}"
            )
    numerator = result.get("write_amplification_numerator", {}).get("median")
    denominator = result.get("write_amplification_denominator", {}).get("median")
    if numerator is not None and denominator:
        result["write_amplification_ratio"] = float(numerator) / float(denominator)
    return result


def runtime_summary(records: list[dict[str, Any]]) -> dict[str, Any]:
    expected = len(records)
    require_sample_ids(records, expected, "runtime")
    measurements = [record["measurement"] for record in records]
    result = {
        "samples": len(records),
        "sample_ids": [record.get("sample") for record in records],
        "metrics": {
            "ready_ns": require_metric_exact(
                (
                    number(nested(m, "timings", "process_start_to_ready_ns"))
                for m in measurements
                if number(nested(m, "timings", "process_start_to_ready_ns")) is not None
                ),
                expected,
                "runtime.ready_ns",
            ),
            "first_lookup_ns": require_metric_exact(
                (
                    number(nested(m, "timings", "first_lookup", "elapsed_ns"))
                for m in measurements
                if number(nested(m, "timings", "first_lookup", "elapsed_ns")) is not None
                ),
                expected,
                "runtime.first_lookup_ns",
            ),
            "anchor_resolution_ns": require_metric_exact(
                (
                    number(nested(m, "timings", "anchor_resolution", "elapsed_ns"))
                for m in measurements
                if number(nested(m, "timings", "anchor_resolution", "elapsed_ns"))
                is not None
                ),
                expected,
                "runtime.anchor_resolution_ns",
            ),
            "workload_ns": require_metric_exact(
                (
                    number(nested(m, "timings", "workload", "elapsed_ns"))
                for m in measurements
                if number(nested(m, "timings", "workload", "elapsed_ns")) is not None
                ),
                expected,
                "runtime.workload_ns",
            ),
            "peak_rss_kib": require_metric_exact(
                (
                    number(nested(record, "process", "maximum_rss_kib"))
                for record in records
                if number(nested(record, "process", "maximum_rss_kib")) is not None
                ),
                expected,
                "runtime.peak_rss_kib",
            ),
            "workload_pss_kib": require_metric_exact(
                (
                    number(nested(m, "smaps", "after_workload", "pss_kib"))
                for m in measurements
                if number(nested(m, "smaps", "after_workload", "pss_kib")) is not None
                ),
                expected,
                "runtime.workload_pss_kib",
            ),
            "workload_private_kib": require_metric_exact(
                (
                    number(nested(m, "smaps", "after_workload", "private_kib"))
                for m in measurements
                if number(nested(m, "smaps", "after_workload", "private_kib"))
                is not None
                ),
                expected,
                "runtime.workload_private_kib",
            ),
            "open_minor_faults": require_metric_exact(
                (
                    number(nested(m, "timings", "open", "faults", "minor"))
                for m in measurements
                if number(nested(m, "timings", "open", "faults", "minor")) is not None
                ),
                expected,
                "runtime.open_minor_faults",
            ),
            "open_major_faults": require_metric_exact(
                (
                    number(nested(m, "timings", "open", "faults", "major"))
                for m in measurements
                if number(nested(m, "timings", "open", "faults", "major")) is not None
                ),
                expected,
                "runtime.open_major_faults",
            ),
        },
        "operations": {},
    }
    resident_growth = []
    for record in records:
        before = record.get("resident_bytes_before")
        after = record.get("resident_bytes_after")
        if isinstance(before, dict) and isinstance(after, dict):
            resident_growth.append(
                sum(max(0, int(after.get(path, 0)) - int(value)) for path, value in before.items())
            )
    if resident_growth:
        result["metrics"]["file_resident_growth_bytes"] = require_metric_exact(
            resident_growth, expected, "runtime.file_resident_growth_bytes"
        )
    operations: dict[str, list[int | float]] = defaultdict(list)
    observed: dict[str, set[int | float]] = defaultdict(set)
    observed_counts: dict[str, int] = defaultdict(int)
    for measurement in measurements:
        seen = set()
        for operation in nested(measurement, "timings", "workload", "operations") or []:
            if not isinstance(operation, dict):
                continue
            name = str(operation.get("name"))
            average = number(operation.get("average_ns"))
            total = number(operation.get("observed_total"))
            if name in seen:
                raise SummaryError(f"duplicate operation in one sample: {name}")
            seen.add(name)
            if average is not None:
                operations[name].append(average)
            if total is not None:
                observed[name].add(total)
                observed_counts[name] += 1
    for name in sorted(operations):
        result["operations"][name] = require_metric_exact(
            operations[name], expected, f"operation.{name}.average_ns"
        )
        result["operations"][name]["observed_totals"] = sorted(observed[name])
        if len(observed[name]) != 1 or observed_counts[name] != expected:
            raise SummaryError(
                f"operation.{name}.observed_total must be present and stable in every sample"
            )
    result["footprint"] = footprint(measurements)
    return result


def production_summary(records: list[dict[str, Any]]) -> dict[str, Any]:
    expected = len(records)
    require_sample_ids(records, expected, "production")
    measurements = [record["measurement"] for record in records]
    result = {
        "samples": len(records),
        "sample_ids": [record.get("sample") for record in records],
        "metrics": {
            "total_ns": require_metric_exact(
                (value
                for m in measurements
                if (value := production_total_ns(m)) is not None),
                expected,
                "production.total_ns",
            ),
            "materialize_ns": require_metric_exact(
                (value
                for m in measurements
                if (value := production_materialize_ns(m)) is not None),
                expected,
                "production.materialize_ns",
            ),
            "write_ns": require_metric_exact(
                (value for m in measurements if (value := production_write_ns(m)) is not None),
                expected,
                "production.write_ns",
            ),
            "artifact_bytes": require_metric_exact(
                (value for m in measurements if (value := artifact_bytes(m)) is not None),
                expected,
                "production.artifact_bytes",
            ),
            "peak_rss_kib": require_metric_exact(
                (number(nested(record, "process", "maximum_rss_kib"))
                for record in records
                if number(nested(record, "process", "maximum_rss_kib")) is not None),
                expected,
                "production.peak_rss_kib",
            ),
        },
        "footprint": footprint(measurements),
        "formation": formation_summary(measurements),
    }
    optional = {
        "serialize_ns": [production_serialize_ns(m) for m in measurements],
        "validate_ns": [production_validate_ns(m) for m in measurements],
    }
    for name, values in optional.items():
        filtered = [value for value in values if value is not None]
        if filtered:
            result["metrics"][name] = require_metric_exact(filtered, expected, name)
    return result


def allocation_phase(measurement: dict[str, Any], producer: bool) -> dict[str, Any] | None:
    if producer:
        return nested(measurement, "allocations", "producer") or nested(
            measurement, "allocation_phases", "total"
        )
    return nested(measurement, "allocations", "entry_to_ready")


def final_allocation(measurement: dict[str, Any]) -> dict[str, Any] | None:
    return nested(measurement, "allocations", "final_snapshot") or nested(
        measurement, "allocation_phases", "final_snapshot"
    )


def allocation_summary(records: list[dict[str, Any]], producer: bool) -> dict[str, Any]:
    expected = len(records)
    require_sample_ids(
        records,
        expected,
        "producer allocation" if producer else "runtime allocation",
    )
    phases = []
    finals = []
    for record in records:
        measurement = record["measurement"]
        phase = allocation_phase(measurement, producer)
        final = final_allocation(measurement)
        if not isinstance(phase, dict) or not isinstance(final, dict):
            raise SummaryError(f"missing allocation phase in {record.get('_source_path')}")
        phases.append(phase)
        finals.append(final)
    return {
        "samples": len(records),
        "sample_ids": [record.get("sample") for record in records],
        "metrics": {
            "allocation_calls": require_metric_exact(
                (number(phase.get("allocation_calls"))
                for phase in phases
                if number(phase.get("allocation_calls")) is not None),
                expected,
                "allocation.allocation_calls",
            ),
            "allocated_bytes": require_metric_exact(
                (number(phase.get("allocated_bytes"))
                for phase in phases
                if number(phase.get("allocated_bytes")) is not None),
                expected,
                "allocation.allocated_bytes",
            ),
            "deallocated_bytes": require_metric_exact(
                (number(phase.get("deallocated_bytes"))
                for phase in phases
                if number(phase.get("deallocated_bytes")) is not None),
                expected,
                "allocation.deallocated_bytes",
            ),
            "final_live_bytes": require_metric_exact(
                (number(final.get("current_live_bytes"))
                for final in finals
                if number(final.get("current_live_bytes")) is not None),
                expected,
                "allocation.final_live_bytes",
            ),
            "peak_live_bytes": require_metric_exact(
                (number(final.get("peak_live_bytes"))
                for final in finals
                if number(final.get("peak_live_bytes")) is not None),
                expected,
                "allocation.peak_live_bytes",
            ),
        },
    }


def four_reader_summary(records: list[dict[str, Any]]) -> dict[str, Any]:
    expected = len(records)
    require_sample_ids(records, expected, "four reader")
    return {
        "samples": len(records),
        "sample_ids": [record.get("sample") for record in records],
        "metrics": {
            name: require_metric_exact(
                (number(nested(record, "aggregate", name))
                for record in records
                if number(nested(record, "aggregate", name)) is not None),
                expected,
                f"four_reader.{name}",
            )
            for name in ("rss_kib", "pss_kib", "private_kib", "shared_kib", "anonymous_kib")
        },
    }


def relative(value: int | float, baseline: int | float) -> dict[str, float]:
    ratio = float(value) / float(baseline) if baseline else 0.0
    return {"ratio": ratio, "delta_percent": (ratio - 1.0) * 100.0}


def baseline_metric(
    baselines: dict[str, Any], baseline: str, area: str, stance: str, name: str
) -> dict[str, Any] | None:
    if area == "runtime":
        return (
            baselines.get(baseline, {})
            .get("runtime", {})
            .get(stance, {})
            .get("metrics", {})
            .get(name)
        )
    if area == "operation":
        return (
            baselines.get(baseline, {})
            .get("runtime", {})
            .get(stance, {})
            .get("operations", {})
            .get(name)
        )
    if area == "production":
        aliases = {"total_ns": "ready_ns", "write_ns": "artifact_write_ns"}
        return (
            baselines.get(baseline, {})
            .get("production", {})
            .get("metrics", {})
            .get(aliases.get(name, name))
        )
    if area == "runtime_allocation":
        aliases = {
            "allocation_calls": "entry_allocation_calls",
            "allocated_bytes": "entry_allocated_bytes",
            "deallocated_bytes": "entry_deallocated_bytes",
        }
        return (
            baselines.get(baseline, {})
            .get("runtime_allocation", {})
            .get("metrics", {})
            .get(aliases.get(name, name))
        )
    if area == "producer_allocation":
        aliases = {
            "allocation_calls": "entry_allocation_calls",
            "allocated_bytes": "entry_allocated_bytes",
            "deallocated_bytes": "entry_deallocated_bytes",
        }
        return (
            baselines.get(baseline, {})
            .get("producer_allocation", {})
            .get("metrics", {})
            .get(aliases.get(name, name))
        )
    if area == "four_reader":
        return baselines.get(baseline, {}).get("four_reader", {}).get("metrics", {}).get(name)
    return None


def add_relatives(candidate: dict[str, Any], baselines: dict[str, Any]) -> None:
    rel: dict[str, Any] = {"to_h0": {}, "to_c0": {}}
    for stance, group in candidate["runtime"].items():
        for label, baseline_key in (("to_h0", "h0"), ("to_c0", "c0")):
            rel[label].setdefault(f"runtime:{stance}", {})
            for name, data in group["metrics"].items():
                baseline = baseline_metric(baselines, baseline_key, "runtime", stance, name)
                if baseline is not None:
                    rel[label][f"runtime:{stance}"][name] = relative(
                        data["median"], baseline["median"]
                    )
        operations = group.get("operations", {})
        for name, data in operations.items():
            c0 = baseline_metric(baselines, "c0", "operation", stance, name)
            if c0 is not None:
                rel["to_c0"].setdefault(f"operations:{stance}", {})[name] = relative(
                    data["median"], c0["median"]
                )
    if candidate.get("production"):
        rel["to_c0"]["production:warm"] = {
            name: relative(data["median"], baseline["median"])
            for name, data in candidate["production"]["metrics"].items()
            if (baseline := baseline_metric(baselines, "c0", "production", "warm", name))
            is not None
        }
    for key, baseline_key in (
        ("runtime_allocation", "runtime_allocation"),
        ("producer_allocation", "producer_allocation"),
        ("four_reader", "four_reader"),
    ):
        if candidate.get(key):
            rel["to_c0"][f"{key}:warm"] = {
                name: relative(data["median"], baseline["median"])
                for name, data in candidate[key]["metrics"].items()
                if (baseline := baseline_metric(baselines, "c0", baseline_key, "warm", name))
                is not None
            }
    candidate["relative"] = rel


def metric_at(candidate: dict[str, Any], area: str, stance: str, name: str) -> dict[str, Any] | None:
    if area == "runtime":
        return candidate.get("runtime", {}).get(stance, {}).get("metrics", {}).get(name)
    if area == "operation":
        return candidate.get("runtime", {}).get(stance, {}).get("operations", {}).get(name)
    if area == "production":
        return candidate.get("production", {}).get("metrics", {}).get(name)
    if area == "runtime_allocation":
        return candidate.get("runtime_allocation", {}).get("metrics", {}).get(name)
    if area == "producer_allocation":
        return candidate.get("producer_allocation", {}).get("metrics", {}).get(name)
    if area == "four_reader":
        return candidate.get("four_reader", {}).get("metrics", {}).get(name)
    return None


def validate_operations(candidate: dict[str, Any], baselines: dict[str, Any]) -> None:
    for stance, group in candidate["runtime"].items():
        expected = baselines["c0"]["runtime"][stance]["operations"]
        actual = group["operations"]
        if set(actual) != set(expected):
            raise SummaryError(
                f"{candidate['backend']} {stance}: operation set mismatch; "
                f"missing={sorted(set(expected) - set(actual))}, "
                f"extra={sorted(set(actual) - set(expected))}"
            )
        for name, expected_operation in expected.items():
            if actual[name].get("observed_totals") != expected_operation.get("observed_totals"):
                raise SummaryError(
                    f"{candidate['backend']} {stance} {name}: observed totals mismatch; "
                    f"expected {expected_operation.get('observed_totals')}, "
                    f"got {actual[name].get('observed_totals')}"
                )


def operation_ceiling(candidate: dict[str, Any], baselines: dict[str, Any]) -> None:
    result = {}
    for stance, group in candidate["runtime"].items():
        failures = []
        entries = {}
        for name, data in group["operations"].items():
            c0 = baselines["c0"]["runtime"][stance]["operations"][name]
            c0_median = c0["median"]
            c0_mad = c0["mad"]
            candidate_mad = data["mad"]
            allowance = max(c0_median * 0.25, 3 * c0_mad, 3 * candidate_mad)
            ceiling = c0_median + allowance
            status = "pass" if data["median"] <= ceiling else "fail"
            entry = {
                "status": status,
                "median": data["median"],
                "c0_median": c0_median,
                "c0_mad": c0_mad,
                "candidate_mad": candidate_mad,
                "ceiling": ceiling,
                "noisy": data["noisy"],
            }
            entries[name] = entry
            if status == "fail":
                failures.append(name)
        result[stance] = {
            "status": "pass" if not failures else "fail",
            "failed_count": len(failures),
            "failed_operations": failures,
            "operations": entries,
        }
    candidate["operation_ceiling"] = result


def fixed_operation_gate(
    candidate: dict[str, Any], stance: str, operation: str, threshold: int
) -> dict[str, Any]:
    item = candidate["runtime"][stance]["operations"][operation]
    status = (
        "inconclusive-noisy"
        if item["noisy"]
        else ("pass" if item["median"] <= threshold else "fail")
    )
    return {
        "status": status,
        "median": item["median"],
        "threshold": threshold,
        "noisy": item["noisy"],
    }


def add_gates(candidate: dict[str, Any], baselines: dict[str, Any]) -> None:
    gates = {}
    noise_exceptions = {
        "warm_first_lookup_ns": "predeclared-absolute-first-lookup-budget",
        "cold_first_lookup_ns": "predeclared-absolute-first-lookup-budget",
    }
    for name, (area, stance, metric_name, threshold) in GATES.items():
        if area == "runtime_max":
            metrics = [
                group["metrics"][metric_name]
                for group in candidate.get("runtime", {}).values()
                if metric_name in group["metrics"]
            ]
            if not metrics:
                gates[name] = {"status": "missing", "threshold": threshold}
                continue
            value = max(item["median"] for item in metrics)
            noisy = any(item["noisy"] for item in metrics)
        else:
            item = metric_at(candidate, area, stance, metric_name)
            if item is None:
                gates[name] = {"status": "missing", "threshold": threshold}
                continue
            value = item["median"]
            noisy = item["noisy"]
        if noisy and name not in noise_exceptions:
            status = "inconclusive-noisy"
        else:
            status = "pass" if value <= threshold else "fail"
        gates[name] = {
            "status": status,
            "median": value,
            "threshold": threshold,
            "noisy": noisy,
        }
        if name in noise_exceptions:
            gates[name]["noise_policy"] = noise_exceptions[name]
    for stance in ("warm", "cold-best-effort"):
        gates[f"{stance}_forward_dictionary_ns"] = {
            "status": (
                "pass"
                if candidate["runtime"][stance]["operations"]["dictionary_by_id"]["median"] <= 10
                else "fail"
            ),
            "median": candidate["runtime"][stance]["operations"]["dictionary_by_id"]["median"],
            "threshold": 10,
            "noisy": candidate["runtime"][stance]["operations"]["dictionary_by_id"]["noisy"],
            "noise_policy": "predeclared-per-operation-mad-envelope",
        }
        gates[f"{stance}_reverse_dictionary_hit_ns"] = fixed_operation_gate(
            candidate, stance, "dictionary_by_value", S83_REVERSE_DICTIONARY_HIT_NS
        )
        gates[f"{stance}_reverse_dictionary_miss_ns"] = fixed_operation_gate(
            candidate,
            stance,
            "dictionary_by_value_miss",
            S83_REVERSE_DICTIONARY_MISS_NS,
        )
        gates[f"{stance}_per_operation_ceiling"] = {
            "status": candidate["operation_ceiling"][stance]["status"],
            "failed_count": candidate["operation_ceiling"][stance]["failed_count"],
            "failed_operations": candidate["operation_ceiling"][stance]["failed_operations"],
            "noise_policy": "predeclared-per-operation-mad-envelope",
        }
    for parity_name in ("storage_parity", "semantic_parity"):
        status = candidate.get(parity_name, {}).get("status", "missing")
        gates[parity_name] = {
            "status": "pass" if status == "pass" else status,
            "source": "supplied" if status != "missing" else "not supplied",
        }
    candidate["gates"] = gates
    candidate["eligibility"] = {
        "eligible": all(gate.get("status") == "pass" for gate in gates.values()),
        "failed_gates": sorted(
            name for name, gate in gates.items() if gate.get("status") == "fail"
        ),
        "inconclusive_noisy_gates": sorted(
            name
            for name, gate in gates.items()
            if gate.get("status") == "inconclusive-noisy"
        ),
        "missing_gates": sorted(
            name for name, gate in gates.items() if gate.get("status") == "missing"
        ),
        "other_blocking_gates": sorted(
            name
            for name, gate in gates.items()
            if gate.get("status")
            not in {"pass", "fail", "inconclusive-noisy", "missing"}
        ),
        "waiver_status": "none",
    }


def parse_status_arguments(values: list[str]) -> dict[str, dict[str, Any]]:
    result = {}
    for value in values:
        if "=" not in value:
            raise SummaryError(f"expected BACKEND=STATUS, got {value!r}")
        backend, status = value.split("=", 1)
        if status == "pass":
            raise SummaryError(
                "explicit parity status cannot claim pass; provide parity raw proof"
            )
        normalized_backend = parity_backend(backend)
        if normalized_backend in result:
            raise SummaryError(
                f"duplicate explicit parity status for {normalized_backend}"
            )
        result[normalized_backend] = {"status": status, "source": "explicit"}
    return result


def parity_records(paths: list[Path], kind: str) -> dict[str, dict[str, Any]]:
    result = {}
    for path in paths:
        for record in read_jsonl(path):
            status = record.get("status")
            if status not in {"ok", "pass"}:
                raise SummaryError(f"{kind} parity failure in {path}: {status}")
            backend = parity_backend(str(record.get("backend", "")))
            if backend in result:
                raise SummaryError(
                    f"duplicate {kind} parity proof for {backend}: "
                    f"{result[backend]['source']} and {path}"
                )
            if kind == "storage":
                passed = (
                    status == "pass"
                    and len(record.get("exit_statuses") or []) == 5
                    and all(exit_status == 0 for exit_status in record.get("exit_statuses") or [])
                    and record.get("content_sha256") == S83_CONTENT_SHA256
                    and record.get("lookup_sha256") == S83_LOOKUP_SHA256
                    and record.get("content_bytes") == 57_486_556
                    and record.get("lookup_bytes") == 88_520_585
                    and record.get("content_records") == 176_793
                    and record.get("lookup_records") == 276_415
                )
                result[backend] = {
                    "status": "pass" if passed else "fail",
                    "candidate_commit": record.get("candidate_commit"),
                    "candidate_branch": record.get("candidate_branch"),
                    "content_sha256": record.get("content_sha256"),
                    "lookup_sha256": record.get("lookup_sha256"),
                    "source": str(path),
                }
            else:
                outputs = record.get("outputs") if isinstance(record.get("outputs"), list) else []
                passed = (
                    status == "pass"
                    and record.get("expected_transcript_sha256") == S83_SEMANTIC_SHA256
                    and record.get("expected_transcript_records") == S83_SEMANTIC_RECORDS
                    and record.get("expected_transcript_size") == S83_SEMANTIC_BYTES
                    and len(record.get("exit_statuses") or []) == 5
                    and all(exit_status == 0 for exit_status in record.get("exit_statuses") or [])
                    and len(outputs) == 5
                    and all(
                        output.get("sha256") == S83_SEMANTIC_SHA256
                        and output.get("size") == S83_SEMANTIC_BYTES
                        and output.get("records") == S83_SEMANTIC_RECORDS
                        and output.get("baseline_byte_equal") is True
                        and output.get("sequential_byte_equal") is True
                        for output in outputs
                    )
                )
                result[backend] = {
                    "status": "pass" if passed else "fail",
                    "candidate_commit": record.get("candidate_commit"),
                    "candidate_branch": record.get("candidate_branch"),
                    "transcript_sha256": record.get("expected_transcript_sha256"),
                    "transcript_records": record.get("expected_transcript_records"),
                    "transcript_bytes": record.get("expected_transcript_size"),
                    "driver_commit": record.get("driver_commit"),
                    "source": str(path),
                }
    return result


def build_summary(records: list[dict[str, Any]], args: argparse.Namespace) -> dict[str, Any]:
    baselines = load_baseline_summary(args.baseline_summary, args.harness_commit)
    validate_official_records(records, args.harness_commit)
    grouped: dict[tuple[str, str, str, str], list[dict[str, Any]]] = defaultdict(list)
    for record in records:
        if not isinstance(record.get("measurement"), dict) and scenario(record) != "aggregate-four-reader-pss":
            continue
        key = (
            runtime_backend(str(record["backend"])),
            scenario(record),
            cache_stance(record),
            str(record.get("candidate_commit", "")),
        )
        grouped[key].append(record)

    runtime_backends = sorted(
        {
            key[0]
            for key, group in grouped.items()
            if key[1] == "runtime"
            and not key[0].endswith("-produce")
            and group
        },
        key=registry_index,
    )
    unknown = [backend for backend in runtime_backends if backend not in REGISTRY_ORDER]
    if unknown:
        raise SummaryError(f"unknown S83 resource backend(s): {unknown}")
    storage = parity_records(args.storage_parity_raw, "storage")
    storage_status = parse_status_arguments(args.storage_parity_status)
    semantic = parity_records(args.semantic_parity_raw, "semantic")
    semantic_status = parse_status_arguments(args.semantic_parity_status)
    for kind, proofs, statuses in (
        ("storage", storage, storage_status),
        ("semantic", semantic, semantic_status),
    ):
        duplicate = sorted(set(proofs) & set(statuses))
        if duplicate:
            raise SummaryError(
                f"duplicate {kind} parity proof/status for backend(s): {duplicate}"
            )
        proofs.update(statuses)

    candidates = []
    for backend in runtime_backends:
        runtime_groups = {}
        commits = {
            record.get("candidate_commit")
            for key, group in grouped.items()
            if key[0] == backend
            for record in group
        }
        if len(commits) != 1:
            raise SummaryError(f"{backend}: mixed candidate commits: {sorted(commits)}")
        commit = str(next(iter(commits)))
        branch = next(
            str(record.get("candidate_branch", ""))
            for key, group in grouped.items()
            if key[0] == backend
            for record in group
            if record.get("candidate_commit") == commit
        )
        for stance in ("warm", "cold-best-effort"):
            group = grouped.get((backend, "runtime", stance, commit), [])
            expected = EXPECTED_COUNTS[f"runtime:{stance}"]
            if len(group) != expected:
                raise SummaryError(f"{backend} runtime {stance}: expected {expected}, got {len(group)}")
            runtime_groups[stance] = runtime_summary(group)
        production_group = grouped.get((backend, "production", "warm", commit), [])
        if len(production_group) != EXPECTED_COUNTS["production:warm"]:
            raise SummaryError(
                f"{backend} production warm: expected 9, got {len(production_group)}"
            )
        runtime_alloc = grouped.get((backend, "allocation-profile", "warm", commit), [])
        producer_alloc = grouped.get((backend, "allocation-profile", "warm", commit), [])
        runtime_alloc = [record for record in runtime_alloc if not str(record["backend"]).endswith("-produce")]
        producer_alloc = [
            record
            for record in grouped.get((backend, "allocation-profile", "warm", commit), [])
            if str(record["backend"]).endswith("-produce")
        ]
        if len(runtime_alloc) != EXPECTED_COUNTS["runtime-allocation:warm"]:
            raise SummaryError(
                f"{backend} runtime allocation: expected 3, got {len(runtime_alloc)}"
            )
        if len(producer_alloc) != EXPECTED_COUNTS["producer-allocation:warm"]:
            raise SummaryError(
                f"{backend} producer allocation: expected 3, got {len(producer_alloc)}"
            )
        four_reader = grouped.get((backend, "aggregate-four-reader-pss", "warm", commit), [])
        if len(four_reader) != EXPECTED_COUNTS["four-reader:warm"]:
            raise SummaryError(f"{backend} four-reader: expected 3, got {len(four_reader)}")

        candidate = {
            "backend": backend,
            "registry_presentation_order": registry_index(backend),
            "registry_order_is_rank": False,
            "candidate_branch": branch,
            "candidate_commit": commit,
            "runtime": runtime_groups,
            "production": production_summary(production_group),
            "runtime_allocation": allocation_summary(runtime_alloc, producer=False),
            "producer_allocation": allocation_summary(producer_alloc, producer=True),
            "four_reader": four_reader_summary(four_reader),
            "storage_parity": storage.get(backend, {"status": "missing"}),
            "semantic_parity": semantic.get(backend, {"status": "missing"}),
        }
        semantic_commit = candidate["semantic_parity"].get("candidate_commit")
        if (
            candidate["semantic_parity"].get("status") == "pass"
            and semantic_commit != commit
        ):
            raise SummaryError(
                f"{backend}: semantic parity commit {semantic_commit} does not match "
                f"resource commit {commit}"
            )
        storage_commit = candidate["storage_parity"].get("candidate_commit")
        if candidate["storage_parity"].get("status") == "pass":
            if storage_commit != commit:
                raise SummaryError(
                    f"{backend}: storage parity commit {storage_commit} does not "
                    f"match resource commit {commit}"
                )
            candidate["storage_parity"]["resource_commit_relation"] = "equal"
        validate_operations(candidate, baselines)
        operation_ceiling(candidate, baselines)
        add_relatives(candidate, baselines)
        add_gates(candidate, baselines)
        candidates.append(candidate)

    eligibility_state = (
        "eligible-candidate-present"
        if any(candidate["eligibility"]["eligible"] for candidate in candidates)
        else "no-candidate-passes-all-frozen-gates"
    )
    return {
        "schema": "hbk-s83-candidate-summary-v1",
        "generated_at_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
        "summarizer_git": git_state(),
        "harness_commit": args.harness_commit,
        "dataset": {
            "id": S83_DATASET,
            "platform_version": S83_PLATFORM,
            "provider_schema_version": 16,
            "extraction_schema_version": 11,
            "sqlite_sha256": S83_SQLITE_SHA256,
            "hbk_sha256": S83_HBK_SHA256,
        },
        "unranked": True,
        "ranked": False,
        "selection": "pending-user-decision",
        "eligibility_state": eligibility_state,
        "baseline_summary": baselines,
        "expected_counts": EXPECTED_COUNTS,
        "candidates": candidates,
    }


def git_state() -> dict[str, Any]:
    try:
        commit = subprocess.check_output(
            ("git", "rev-parse", "HEAD"), text=True, stderr=subprocess.DEVNULL
        ).strip()
        dirty = bool(
            subprocess.check_output(
                ("git", "status", "--porcelain"), text=True, stderr=subprocess.DEVNULL
            ).strip()
        )
        return {"commit": commit, "working_tree_dirty": dirty}
    except (OSError, subprocess.CalledProcessError):
        return {"commit": None, "working_tree_dirty": None}


def median_at(group: dict[str, Any], *path: str) -> Any:
    value: Any = group
    for key in path:
        value = value.get(key, {}) if isinstance(value, dict) else {}
    return value.get("median") if isinstance(value, dict) else None


def fmt_ms(value: Any) -> str:
    return "n/a" if value is None else f"{float(value) / 1_000_000:.3f}"


def fmt_us(value: Any) -> str:
    return "n/a" if value is None else f"{float(value) / 1_000:.3f}"


def fmt_mib_from_kib(value: Any) -> str:
    return "n/a" if value is None else f"{float(value) / 1024:.2f}"


def fmt_mib_from_bytes(value: Any) -> str:
    return "n/a" if value is None else f"{float(value) / 1024 / 1024:.2f}"


def render_markdown(summary: dict[str, Any]) -> str:
    lines = [
        "# S83 Candidate Evidence",
        "",
        f"Frozen harness commit: `{summary['harness_commit']}`.",
        "",
        "Rows are evidence only. This report records no ordering or canonical choice.",
        "Registry presentation order is fixed by the hypothesis registry and is not a rank.",
        f"Eligibility state: `{summary['eligibility_state']}`.",
        "",
        "| Backend | Commit | Warm ready ms | Cold ready ms | Warm first lookup us | Warm workload ms | Warm PSS MiB | Cold PSS MiB |",
        "| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |",
    ]
    for candidate in summary["candidates"]:
        warm = candidate["runtime"]["warm"]
        cold = candidate["runtime"]["cold-best-effort"]
        lines.append(
            f"| {candidate['backend']} | `{candidate['candidate_commit'][:12]}` | "
            f"{fmt_ms(median_at(warm, 'metrics', 'ready_ns'))} | "
            f"{fmt_ms(median_at(cold, 'metrics', 'ready_ns'))} | "
            f"{fmt_us(median_at(warm, 'metrics', 'first_lookup_ns'))} | "
            f"{fmt_ms(median_at(warm, 'metrics', 'workload_ns'))} | "
            f"{fmt_mib_from_kib(median_at(warm, 'metrics', 'workload_pss_kib'))} | "
            f"{fmt_mib_from_kib(median_at(cold, 'metrics', 'workload_pss_kib'))} |"
        )
    lines.extend(
        [
            "",
            "## Production",
            "",
            "| Backend | N | Total ms | Materialize ms | Serialize ms | Validate ms | Write ms | Artifact MiB | Peak RSS MiB |",
            "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
        ]
    )
    for candidate in summary["candidates"]:
        group = candidate["production"]
        metrics = group["metrics"]
        lines.append(
            f"| {candidate['backend']} | {group['samples']} | "
            f"{fmt_ms(median_at(group, 'metrics', 'total_ns'))} | "
            f"{fmt_ms(median_at(group, 'metrics', 'materialize_ns'))} | "
            f"{fmt_ms(median_at(group, 'metrics', 'serialize_ns'))} | "
            f"{fmt_ms(median_at(group, 'metrics', 'validate_ns'))} | "
            f"{fmt_ms(median_at(group, 'metrics', 'write_ns'))} | "
            f"{fmt_mib_from_bytes(metrics['artifact_bytes']['median'])} | "
            f"{fmt_mib_from_kib(metrics['peak_rss_kib']['median'])} |"
        )
    lines.extend(
        [
            "",
            "## Allocation And Four Readers",
            "",
            "| Backend | Runtime alloc calls | Runtime allocated MiB | Producer alloc calls | Producer allocated MiB | Four-reader PSS MiB |",
            "| --- | ---: | ---: | ---: | ---: | ---: |",
        ]
    )
    for candidate in summary["candidates"]:
        runtime_alloc = candidate["runtime_allocation"]["metrics"]
        producer_alloc = candidate["producer_allocation"]["metrics"]
        four = candidate["four_reader"]["metrics"]
        lines.append(
            f"| {candidate['backend']} | "
            f"{runtime_alloc['allocation_calls']['median']} | "
            f"{fmt_mib_from_bytes(runtime_alloc['allocated_bytes']['median'])} | "
            f"{producer_alloc['allocation_calls']['median']} | "
            f"{fmt_mib_from_bytes(producer_alloc['allocated_bytes']['median'])} | "
            f"{fmt_mib_from_kib(four['pss_kib']['median'])} |"
        )
    lines.extend(
        [
            "",
            "## Footprint",
            "",
            "| Backend | Artifact MiB | Section MiB | Dictionary MiB | Index MiB | Dictionary text MiB | Archive index fixed MiB |",
            "| --- | ---: | ---: | ---: | ---: | ---: | ---: |",
        ]
    )
    for candidate in summary["candidates"]:
        fp = candidate["production"]["footprint"] or candidate["runtime"]["warm"]["footprint"]
        lines.append(
            f"| {candidate['backend']} | "
            f"{fmt_mib_from_bytes(median_at({'metrics': fp}, 'metrics', 'artifact_bytes'))} | "
            f"{fmt_mib_from_bytes(median_at({'metrics': fp}, 'metrics', 'section_bytes'))} | "
            f"{fmt_mib_from_bytes(median_at({'metrics': fp}, 'metrics', 'dictionary_bytes'))} | "
            f"{fmt_mib_from_bytes(median_at({'metrics': fp}, 'metrics', 'index_bytes'))} | "
            f"{fmt_mib_from_bytes(median_at({'metrics': fp}, 'metrics', 'dictionary_text_bytes'))} | "
            f"{fmt_mib_from_bytes(median_at({'metrics': fp}, 'metrics', 'sorted_index_estimated_fixed_bytes'))} |"
        )
    lines.extend(
        [
            "",
            "## Hypothesis-specific Footprint",
            "",
            "| Backend | Mapped hash MiB | Hash bucket MiB | Hash tables | Hash groups | Hash buckets | Max probe | Record head MiB | Nested arena MiB |",
            "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
        ]
    )
    for candidate in summary["candidates"]:
        fp = candidate["production"]["footprint"] or candidate["runtime"]["warm"]["footprint"]
        wrapped = {"metrics": fp}
        lines.append(
            f"| {candidate['backend']} | "
            f"{fmt_mib_from_bytes(median_at(wrapped, 'metrics', 'mapped_hash_bytes'))} | "
            f"{fmt_mib_from_bytes(median_at(wrapped, 'metrics', 'mapped_hash_bucket_bytes'))} | "
            f"{median_at(wrapped, 'metrics', 'mapped_hash_tables') or 'n/a'} | "
            f"{median_at(wrapped, 'metrics', 'mapped_hash_groups') or 'n/a'} | "
            f"{median_at(wrapped, 'metrics', 'mapped_hash_buckets') or 'n/a'} | "
            f"{median_at(wrapped, 'metrics', 'mapped_hash_max_probe') or 'n/a'} | "
            f"{fmt_mib_from_bytes(median_at(wrapped, 'metrics', 'record_head_bytes'))} | "
            f"{fmt_mib_from_bytes(median_at(wrapped, 'metrics', 'nested_arena_bytes'))} |"
        )
    lines.extend(
        [
            "",
            "## Direct Formation",
            "",
            "| Backend | Strategy | Monolithic artifact buffer | At most one completed section | Logical MiB written | Peak section MiB | Peak tracked working MiB | Write amplification |",
            "| --- | --- | --- | --- | ---: | ---: | ---: | ---: |",
        ]
    )
    for candidate in summary["candidates"]:
        formation = candidate["production"].get("formation", {})
        lines.append(
            f"| {candidate['backend']} | "
            f"{formation.get('strategy', 'n/a')} | "
            f"{formation.get('retains_monolithic_artifact_buffer', 'n/a')} | "
            f"{formation.get('retains_at_most_one_completed_section_buffer', 'n/a')} | "
            f"{fmt_mib_from_bytes(median_at({'metrics': formation}, 'metrics', 'logical_bytes_written'))} | "
            f"{fmt_mib_from_bytes(median_at({'metrics': formation}, 'metrics', 'peak_section_buffer_bytes'))} | "
            f"{fmt_mib_from_bytes(median_at({'metrics': formation}, 'metrics', 'peak_working_buffer_bytes'))} | "
            f"{formation.get('write_amplification_ratio', 'n/a')} |"
        )
    lines.extend(
        [
            "",
            "## Eligibility",
            "",
            "| Backend | Eligible | Failed gates | Inconclusive noisy gates | Missing gates | Other blockers | Waiver |",
            "| --- | --- | --- | --- | --- | --- | --- |",
        ]
    )
    for candidate in summary["candidates"]:
        eligibility = candidate["eligibility"]
        lines.append(
            f"| {candidate['backend']} | {eligibility['eligible']} | "
            f"{', '.join(eligibility['failed_gates']) or 'none'} | "
            f"{', '.join(eligibility['inconclusive_noisy_gates']) or 'none'} | "
            f"{', '.join(eligibility['missing_gates']) or 'none'} | "
            f"{', '.join(eligibility['other_blocking_gates']) or 'none'} | "
            f"{eligibility['waiver_status']} |"
        )
    lines.extend(
        [
            "",
            "## Frozen Gates",
            "",
            "| Backend | Gate | Status | Noisy | Median | Threshold | Failed count | Failed operations |",
            "| --- | --- | --- | --- | ---: | ---: | ---: | --- |",
        ]
    )
    for candidate in summary["candidates"]:
        for name in sorted(candidate["gates"]):
            gate = candidate["gates"][name]
            lines.append(
                f"| {candidate['backend']} | {name} | {gate['status']} | "
                f"{gate.get('noisy', 'n/a')} | "
                f"{gate.get('median', 'n/a')} | {gate.get('threshold', 'n/a')} | "
                f"{gate.get('failed_count', 'n/a')} | "
                f"{', '.join(gate.get('failed_operations', [])) or 'n/a'} |"
            )
    lines.append("")
    return "\n".join(lines)


def main() -> int:
    args = parse_args()
    if args.harness_commit != S83_HARNESS_COMMIT:
        print(
            f"error: this postprocessor is frozen for {S83_HARNESS_COMMIT}, "
            f"got {args.harness_commit}",
            file=sys.stderr,
        )
        return 2
    try:
        records = [record for path in args.raw_jsonl for record in read_jsonl(path)]
        summary = build_summary(records, args)
        args.json_path.parent.mkdir(parents=True, exist_ok=True)
        args.markdown_path.parent.mkdir(parents=True, exist_ok=True)
        args.json_path.write_text(
            json.dumps(summary, ensure_ascii=False, indent=2) + "\n",
            encoding="utf-8",
        )
        args.markdown_path.write_text(render_markdown(summary), encoding="utf-8")
    except (OSError, json.JSONDecodeError, SummaryError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
