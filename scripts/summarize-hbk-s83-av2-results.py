#!/usr/bin/env python3
"""Summarize frozen S83-AV2 raw evidence without ranking candidates."""

from __future__ import annotations

import argparse
import importlib.util
import json
import statistics
import sys
from collections import defaultdict
from pathlib import Path
from typing import Any, Iterable, Sequence


SCRIPTS = Path(__file__).resolve().parent
_SPEC = importlib.util.spec_from_file_location(
    "hbk_s83_av2_evidence_contract",
    SCRIPTS / "_hbk_s83_av2_evidence_contract.py",
)
if _SPEC is None or _SPEC.loader is None:
    raise RuntimeError("cannot load S83-AV2 benchmark contract")
contract = importlib.util.module_from_spec(_SPEC)
sys.modules[_SPEC.name] = contract
_SPEC.loader.exec_module(contract)

RAW_SCHEMA = contract.RAW_SCHEMA
SUMMARY_SCHEMA = contract.SUMMARY_SCHEMA
REPORT_SCHEMA = contract.REPORT_SCHEMA
WORKLOAD_VERSION = contract.WORKLOAD_VERSION
DATASET = contract.DATASET
BACKENDS = contract.BACKENDS
DECISION_ROLES = contract.DECISION_ROLES
OPERATIONS = contract.OPERATIONS
AVAILABILITY_CONTEXTS = contract.AVAILABILITY_CONTEXTS
CACHE_STANCES = contract.CACHE_STANCES
NOISE_LIMIT = 0.05


class SummaryError(RuntimeError):
    """S83-AV2 raw evidence is incomplete or inconsistent."""


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("raw_jsonl", nargs="+", type=Path)
    parser.add_argument("--expected-samples", type=positive_int, default=contract.DEFAULT_SAMPLES)
    parser.add_argument("--json", dest="json_path", type=Path, required=True)
    parser.add_argument("--markdown", dest="markdown_path", type=Path, required=True)
    return parser.parse_args(argv)


def positive_int(value: str) -> int:
    parsed = int(value)
    if parsed <= 0:
        raise argparse.ArgumentTypeError("value must be greater than zero")
    return parsed


def validate_expected_samples(value: int) -> None:
    if value != contract.DEFAULT_SAMPLES:
        raise SummaryError(
            f"expected samples must remain frozen at {contract.DEFAULT_SAMPLES} for hbk-s83-av2/v1"
        )


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
                    raise SummaryError(f"{path}:{line_number}: expected object")
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
        raise SummaryError(f"{path} must be numeric")
    return value


def median(values: Sequence[int | float]) -> float:
    return float(statistics.median(values))


def mad(values: Sequence[int | float]) -> float:
    med = median(values)
    return float(statistics.median(abs(value - med) for value in values))


def operation_contexts(operation: str) -> tuple[str | None, ...]:
    return contract.operation_contexts(operation)


METRICS: dict[str, tuple[str, ...]] = {
    "entry_to_ready_ns": ("measurement", "timings", "entry_to_ready", "elapsed_ns"),
    "first_operation_ns": ("measurement", "timings", "first_operation", "elapsed_ns"),
    "steady_workload_ns": ("measurement", "timings", "steady_workload", "elapsed_ns"),
    "steady_average_ns": ("measurement", "timings", "steady_workload", "average_ns"),
    "steady_ns_per_query": ("measurement", "timings", "steady_workload", "ns_per_query"),
    "steady_ns_per_object": ("measurement", "timings", "steady_workload", "ns_per_object"),
    "steady_minor_faults": ("measurement", "faults", "steady_workload", "minor"),
    "steady_major_faults": ("measurement", "faults", "steady_workload", "major"),
    "steady_allocation_calls": ("measurement", "allocations", "steady_workload", "allocation_calls"),
    "steady_allocated_bytes": ("measurement", "allocations", "steady_workload", "allocated_bytes"),
    "memory_logical_bytes": ("measurement", "memory", "logical_bytes"),
    "memory_capacity_bytes": ("measurement", "memory", "capacity_bytes"),
    "payload_canonical_bytes": ("measurement", "operation_data", "canonical_payload_bytes_touched"),
}


RAW_KEYS = frozenset(
    {
        "schema",
        "dataset",
        "platform_version",
        "provider_schema_version",
        "extraction_schema_version",
        "hbk_sha256",
        "provider_sha256",
        "backend",
        "decision_role",
        "candidate_commit",
        "candidate_branch",
        "worktree",
        "executable_artifact",
        "harness_commit",
        "harness_branch",
        "harness_file_sha256",
        "manifest_sha256",
        "manifest_bytes",
        "host",
        "orchestration_version",
        "backend_registry",
        "operation_registry",
        "availability_context_registry",
        "cache_stance_registry",
        "planned_samples_per_row",
        "declared_file_artifacts",
        "operation",
        "availability_context",
        "cache_stance",
        "sample",
        "iterations",
        "status",
        "command_template",
        "command",
        "declared_files",
        "preparation",
        "machine_state_before",
        "machine_state_after",
        "stdout_log",
        "stderr_log",
        "stderr_sha256",
        "h0_parity_sha256",
        "measurement",
        "_source_path",
        "_source_line",
    }
)
RAW_SOURCE_KEYS = frozenset({"_source_path", "_source_line"})


def validate_raw_record(record: dict[str, Any]) -> None:
    contract.reject_forbidden_fields(record)
    actual_without_source = set(record) - RAW_SOURCE_KEYS
    expected_without_source = RAW_KEYS - RAW_SOURCE_KEYS
    if actual_without_source != expected_without_source:
        raise SummaryError(
            f"raw schema keys differ: missing={sorted(expected_without_source - actual_without_source)}, "
            f"unexpected={sorted(actual_without_source - expected_without_source)}"
        )
    expected = {
        "schema": RAW_SCHEMA,
        "dataset": DATASET,
        "platform_version": contract.PLATFORM_VERSION,
        "provider_schema_version": contract.PROVIDER_SCHEMA_VERSION,
        "extraction_schema_version": contract.EXTRACTION_SCHEMA_VERSION,
        "hbk_sha256": contract.HBK_SHA256,
        "provider_sha256": contract.PROVIDER_SHA256,
        "orchestration_version": contract.ORCHESTRATION_VERSION,
        "backend_registry": list(BACKENDS),
        "operation_registry": list(OPERATIONS),
        "availability_context_registry": list(AVAILABILITY_CONTEXTS),
        "cache_stance_registry": list(CACHE_STANCES),
        "status": "ok",
    }
    for key, expected_value in expected.items():
        if record[key] != expected_value:
            raise SummaryError(f"raw.{key} differs from frozen S83-AV2 contract")
    backend = record["backend"]
    operation = record["operation"]
    context = record["availability_context"]
    if backend not in BACKENDS:
        raise SummaryError(f"unknown backend: {backend}")
    if record["decision_role"] != DECISION_ROLES[backend]:
        raise SummaryError(f"{backend}: invalid decision_role")
    contract.validate_artifact(record["executable_artifact"], "raw.executable_artifact")
    if operation not in OPERATIONS:
        raise SummaryError(f"unknown operation: {operation}")
    if context not in operation_contexts(operation):
        raise SummaryError(f"{operation}: invalid availability_context {context!r}")
    if record["cache_stance"] not in CACHE_STANCES:
        raise SummaryError("invalid cache stance")
    sample = record["sample"]
    if isinstance(sample, bool) or not isinstance(sample, int) or sample <= 0:
        raise SummaryError("sample must be positive integer")
    if "canonical_transcript" in json.dumps(record["measurement"], ensure_ascii=False):
        raise SummaryError("raw performance must not contain canonical transcript")
    pseudo_backend = contract.Backend(
        backend=backend,
        decision_role=record["decision_role"],
        worktree=Path(record["worktree"]),
        command=tuple(record["command_template"]),
        declared_files=tuple(Path(path) for path in record["declared_files"]),
        declared_file_artifacts=tuple(record["declared_file_artifacts"]),
        executable=Path(record["executable_artifact"]["path"]),
        executable_artifact=record["executable_artifact"],
        commit=record["candidate_commit"],
        branch=record["candidate_branch"],
    )
    _transcript, stripped = contract.validate_report(
        record["measurement"],
        pseudo_backend,
        operation,
        context,
        record["iterations"],
        record["manifest_sha256"],
        record["manifest_bytes"],
    )
    if _transcript is not None:
        raise SummaryError("raw performance must not include parity transcript")
    if stripped != record["measurement"]:
        raise SummaryError("measurement normalization changed raw payload")


def validate_complete_matrix(records: Sequence[dict[str, Any]], expected_samples: int) -> None:
    seen: set[tuple[str, str, str | None, str, int]] = set()
    expected: set[tuple[str, str, str | None, str, int]] = set()
    for backend in BACKENDS:
        for operation in OPERATIONS:
            for context in operation_contexts(operation):
                for stance in CACHE_STANCES:
                    for sample in range(1, expected_samples + 1):
                        expected.add((backend, operation, context, stance, sample))
    for record in records:
        key = (
            record["backend"],
            record["operation"],
            record["availability_context"],
            record["cache_stance"],
            record["sample"],
        )
        if key in seen:
            raise SummaryError(f"duplicate matrix row: {key}")
        seen.add(key)
    missing = expected - seen
    unexpected = seen - expected
    if missing or unexpected:
        raise SummaryError(f"incomplete S83-AV2 matrix: missing={len(missing)}, unexpected={len(unexpected)}")


def summarize_group(records: Sequence[dict[str, Any]], h0_values: dict[str, float]) -> dict[str, Any]:
    metrics: dict[str, dict[str, Any]] = {}
    for metric, path in METRICS.items():
        values = [number(nested(record, path), ".".join(path)) for record in records if nested(record, path) is not None]
        if not values:
            continue
        med = median(values)
        metric_mad = mad(values)
        noise = 0.0 if med == 0 else metric_mad / med
        h0 = h0_values.get(metric)
        metrics[metric] = {
            "median": med,
            "mad": metric_mad,
            "noise": noise,
            "noise_status": "noisy" if noise > NOISE_LIMIT else "ok",
            "h0_ratio": None if h0 in (None, 0) else med / h0,
        }
    return metrics


def build_summary(records: Sequence[dict[str, Any]], expected_samples: int) -> dict[str, Any]:
    for record in records:
        validate_raw_record(record)
    validate_complete_matrix(records, expected_samples)

    grouped: dict[tuple[str, str, str | None, str], list[dict[str, Any]]] = defaultdict(list)
    for record in records:
        grouped[(record["backend"], record["operation"], record["availability_context"], record["cache_stance"])].append(record)

    h0_metrics: dict[tuple[str, str | None, str], dict[str, float]] = {}
    rows: list[dict[str, Any]] = []
    for (backend, operation, context, stance), group in sorted(grouped.items(), key=lambda item: (BACKENDS.index(item[0][0]), OPERATIONS.index(item[0][1]), str(item[0][2]), CACHE_STANCES.index(item[0][3]))):
        if len(group) != expected_samples:
            raise SummaryError(f"{backend}/{operation}/{context}/{stance}: expected {expected_samples} samples")
        if backend == "S83-H0":
            h0_values = {}
        else:
            h0_values = h0_metrics[(operation, context, stance)]
        metrics = summarize_group(group, h0_values)
        if backend == "S83-H0":
            h0_metrics[(operation, context, stance)] = {
                name: metric["median"] for name, metric in metrics.items()
            }
            for metric in metrics.values():
                metric["h0_ratio"] = 1.0
        rows.append(
            {
                "backend": backend,
                "decision_role": DECISION_ROLES[backend],
                "operation": operation,
                "availability_context": context,
                "cache_stance": stance,
                "samples": expected_samples,
                "metrics": metrics,
            }
        )
    return {
        "schema": SUMMARY_SCHEMA,
        "workload_version": WORKLOAD_VERSION,
        "dataset": DATASET,
        "backend_registry": list(BACKENDS),
        "operation_registry": list(OPERATIONS),
        "availability_context_registry": list(AVAILABILITY_CONTEXTS),
        "cache_stance_registry": list(CACHE_STANCES),
        "expected_samples": expected_samples,
        "noise_limit": NOISE_LIMIT,
        "rows": rows,
        "notes": [
            "S83-AV2 is descriptive evidence only.",
            "Rows are not ranked and no candidate is selected.",
        ],
    }


def markdown_summary(summary: dict[str, Any]) -> str:
    lines = [
        "# S83-AV2 Summary",
        "",
        "Descriptive evidence only. No ranking, winner, recommendation, or candidate selection is encoded.",
        "",
        "| Backend | Operation | Context | Stance | Samples | Steady median ns | Steady MAD | H0 ratio | Noise |",
        "| --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: |",
    ]
    for row in summary["rows"]:
        steady = row["metrics"].get("steady_workload_ns", {})
        lines.append(
            "| {backend} | {operation} | {context} | {stance} | {samples} | {median:.3f} | {mad:.3f} | {ratio} | {noise:.6f} |".format(
                backend=row["backend"],
                operation=row["operation"],
                context=row["availability_context"] or "",
                stance=row["cache_stance"],
                samples=row["samples"],
                median=steady.get("median", 0.0),
                mad=steady.get("mad", 0.0),
                ratio="" if steady.get("h0_ratio") is None else f"{steady['h0_ratio']:.6f}",
                noise=steady.get("noise", 0.0),
            )
        )
    lines.append("")
    return "\n".join(lines)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        validate_expected_samples(args.expected_samples)
        records = read_jsonl(args.raw_jsonl)
        summary = build_summary(records, args.expected_samples)
        contract.reject_forbidden_fields(summary)
        args.json_path.parent.mkdir(parents=True, exist_ok=True)
        args.markdown_path.parent.mkdir(parents=True, exist_ok=True)
        args.json_path.write_text(json.dumps(summary, ensure_ascii=False, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        args.markdown_path.write_text(markdown_summary(summary), encoding="utf-8")
        print(args.json_path)
        return 0
    except (SummaryError, contract.EvidenceError, OSError, json.JSONDecodeError) as error:
        print(f"S83-AV2 summary failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
