#!/usr/bin/env python3
"""Summarize S83-AV1 raw evidence without selecting a candidate."""

from __future__ import annotations

import argparse
import json
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
    returned_objects = nested(measurement, ("counts", "returned_objects"))
    if transcript.get("item_count") != returned_objects:
        raise SummaryError(f"{source}: transcript item count differs from measurement")
    if nested(measurement, ("allocations", "enabled")) is not True:
        raise SummaryError(f"{source}: allocation instrumentation is not enabled")
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
