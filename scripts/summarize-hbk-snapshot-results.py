#!/usr/bin/env python3
"""Summarize versioned HBK snapshot JSONL without ranking candidates."""

from __future__ import annotations

import argparse
import json
import statistics
from collections import defaultdict
from pathlib import Path
from typing import Any, Iterable


METRICS: dict[str, tuple[str, ...]] = {
    "ready_ns": ("measurement", "timings", "process_start_to_ready_ns"),
    "first_lookup_ns": ("measurement", "timings", "first_lookup", "elapsed_ns"),
    "anchor_resolution_ns": (
        "measurement",
        "timings",
        "anchor_resolution",
        "elapsed_ns",
    ),
    "workload_ns": ("measurement", "timings", "workload", "elapsed_ns"),
    "peak_rss_kib": ("process", "maximum_rss_kib"),
    "open_rss_kib": ("measurement", "smaps", "after_open", "rss_kib"),
    "open_pss_kib": ("measurement", "smaps", "after_open", "pss_kib"),
    "open_private_kib": ("measurement", "smaps", "after_open", "private_kib"),
    "workload_rss_kib": ("measurement", "smaps", "after_workload", "rss_kib"),
    "workload_pss_kib": ("measurement", "smaps", "after_workload", "pss_kib"),
    "workload_private_kib": (
        "measurement",
        "smaps",
        "after_workload",
        "private_kib",
    ),
    "open_minor_faults": (
        "measurement",
        "timings",
        "open",
        "faults",
        "minor",
    ),
    "open_major_faults": (
        "measurement",
        "timings",
        "open",
        "faults",
        "major",
    ),
    "snapshot_heap_bytes": (
        "measurement",
        "snapshot",
        "estimated_heap_bytes",
    ),
    "logical_payload_bytes": (
        "measurement",
        "snapshot",
        "logical_payload_bytes",
    ),
    "artifact_bytes": ("measurement", "cache", "bytes"),
}

ALLOCATION_METRICS: dict[str, tuple[str, ...]] = {
    "entry_allocation_calls": (
        "measurement",
        "allocations",
        "entry_to_ready",
        "allocation_calls",
    ),
    "entry_reallocation_calls": (
        "measurement",
        "allocations",
        "entry_to_ready",
        "reallocation_calls",
    ),
    "entry_allocated_bytes": (
        "measurement",
        "allocations",
        "entry_to_ready",
        "allocated_bytes",
    ),
    "entry_deallocated_bytes": (
        "measurement",
        "allocations",
        "entry_to_ready",
        "deallocated_bytes",
    ),
    "anchor_allocated_bytes": (
        "measurement",
        "allocations",
        "anchor_resolution",
        "allocated_bytes",
    ),
    "final_live_bytes": (
        "measurement",
        "allocations",
        "final_snapshot",
        "current_live_bytes",
    ),
    "peak_live_bytes": (
        "measurement",
        "allocations",
        "final_snapshot",
        "peak_live_bytes",
    ),
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("raw_jsonl", type=Path)
    parser.add_argument("--harness-commit", required=True)
    parser.add_argument("--json", dest="json_path", type=Path, required=True)
    parser.add_argument("--markdown", dest="markdown_path", type=Path, required=True)
    return parser.parse_args()


def read_records(path: Path) -> list[dict[str, Any]]:
    records = []
    with path.open(encoding="utf-8") as source:
        for line_number, line in enumerate(source, 1):
            if not line.strip():
                continue
            value = json.loads(line)
            if not isinstance(value, dict):
                raise ValueError(f"{path}:{line_number}: expected JSON object")
            records.append(value)
    return records


def nested(record: dict[str, Any], path: tuple[str, ...]) -> int | float | None:
    value: Any = record
    for key in path:
        if not isinstance(value, dict) or key not in value:
            return None
        value = value[key]
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        return None
    return value


def median_mad(values: Iterable[int | float]) -> dict[str, float | int] | None:
    values = list(values)
    if not values:
        return None
    median = statistics.median(values)
    mad = statistics.median(abs(value - median) for value in values)
    ratio = 0.0 if median == 0 else float(mad / median)
    return {
        "samples": len(values),
        "median": median,
        "mad": mad,
        "mad_ratio": ratio,
        "noisy": ratio > 0.05,
    }


def summarize_group(records: list[dict[str, Any]]) -> dict[str, Any]:
    summary: dict[str, Any] = {
        "samples": len(records),
        "sample_ids": [record["sample"] for record in records],
        "candidate_commits": sorted(
            {str(record.get("candidate_commit", "")) for record in records}
        ),
        "candidate_branches": sorted(
            {str(record.get("candidate_branch", "")) for record in records}
        ),
        "metrics": {},
        "operations": {},
    }
    for name, path in METRICS.items():
        metric = median_mad(
            value for record in records if (value := nested(record, path)) is not None
        )
        if metric is not None:
            summary["metrics"][name] = metric

    operations: dict[str, list[int | float]] = defaultdict(list)
    observations: dict[str, set[int | float]] = defaultdict(set)
    for record in records:
        workload = (
            record.get("measurement", {}).get("timings", {}).get("workload") or {}
        )
        for operation in workload.get("operations", []):
            operations[operation["name"]].append(operation["average_ns"])
            observations[operation["name"]].add(operation["observed_total"])
    for name in sorted(operations):
        summary["operations"][name] = median_mad(operations[name])
        summary["operations"][name]["observed_totals"] = sorted(observations[name])
    return summary


def summarize_allocation_group(records: list[dict[str, Any]]) -> dict[str, Any]:
    summary: dict[str, Any] = {"samples": len(records), "metrics": {}}
    for name, path in ALLOCATION_METRICS.items():
        metric = median_mad(
            value for record in records if (value := nested(record, path)) is not None
        )
        if metric is not None:
            summary["metrics"][name] = metric
    return summary


def relative_percent(value: float, baseline: float) -> float | None:
    if baseline == 0:
        return None
    return (value - baseline) * 100.0 / baseline


def add_relatives(groups: dict[str, dict[str, Any]]) -> None:
    for key, group in groups.items():
        _, stance = key.split("|", 1)
        baseline = groups.get(f"sql-owned|{stance}")
        if baseline is None:
            continue
        relatives = {}
        for metric, result in group["metrics"].items():
            base_result = baseline["metrics"].get(metric)
            if base_result is None:
                continue
            relatives[metric] = relative_percent(
                float(result["median"]), float(base_result["median"])
            )
        group["relative_to_sql_percent"] = relatives


def format_ms(value_ns: int | float | None) -> str:
    return "—" if value_ns is None else f"{float(value_ns) / 1_000_000:.3f}"


def format_us(value_ns: int | float | None) -> str:
    return "—" if value_ns is None else f"{float(value_ns) / 1_000:.3f}"


def format_mib(value_kib: int | float | None) -> str:
    return "—" if value_kib is None else f"{float(value_kib) / 1024:.2f}"


def metric_median(group: dict[str, Any], name: str) -> int | float | None:
    result = group["metrics"].get(name)
    return None if result is None else result["median"]


def render_markdown(
    harness_commit: str,
    groups: dict[str, dict[str, Any]],
    parity_records: list[dict[str, Any]],
    aggregate_records: list[dict[str, Any]],
    allocation_groups: dict[str, dict[str, Any]],
) -> str:
    lines = [
        "# HBK Snapshot Comparison (Unranked)",
        "",
        f"Frozen harness commit: `{harness_commit}`.",
        "",
        "Rows are evidence only. This report does not assign a score, rank or winner.",
        "",
        "| Backend | Cache stance | N | Ready ms | First lookup µs | Workload ms | Peak RSS MiB | Post-workload PSS MiB | Post-workload private MiB | Open minor faults |",
        "| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
    ]
    for key in sorted(groups):
        backend, stance = key.split("|", 1)
        group = groups[key]
        lines.append(
            "| "
            + " | ".join(
                [
                    backend,
                    stance,
                    str(group["samples"]),
                    format_ms(metric_median(group, "ready_ns")),
                    format_us(metric_median(group, "first_lookup_ns")),
                    format_ms(metric_median(group, "workload_ns")),
                    format_mib(metric_median(group, "peak_rss_kib")),
                    format_mib(metric_median(group, "workload_pss_kib")),
                    format_mib(metric_median(group, "workload_private_kib")),
                    str(metric_median(group, "open_minor_faults") or "—"),
                ]
            )
            + " |"
        )

    lines.extend(["", "## Parity evidence", ""])
    if parity_records:
        lines.extend(
            [
                "| Backend | Status | Content SHA-256 | Lookup SHA-256 |",
                "| --- | --- | --- | --- |",
            ]
        )
        for record in parity_records:
            lines.append(
                f"| {record.get('backend', '—')} | {record.get('status', '—')} | "
                f"`{record.get('content_sha256', '—')}` | "
                f"`{record.get('lookup_sha256', '—')}` |"
            )
    else:
        lines.append("No parity record for this harness commit.")

    lines.extend(["", "## Aggregate four-reader PSS", ""])
    if aggregate_records:
        lines.extend(
            [
                "| Backend | Aggregate PSS MiB | Aggregate private MiB |",
                "| --- | ---: | ---: |",
            ]
        )
        for record in aggregate_records:
            aggregate = record["aggregate"]
            lines.append(
                f"| {record['backend']} | "
                f"{float(aggregate['pss_kib']) / 1024:.2f} | "
                f"{float(aggregate['private_kib']) / 1024:.2f} |"
            )
    else:
        lines.append("No aggregate-reader record for this harness commit.")

    lines.extend(["", "## Allocation profiles", ""])
    if allocation_groups:
        lines.extend(
            [
                "| Backend | N | Entry allocations | Entry allocated MiB | Final live MiB | Peak live MiB |",
                "| --- | ---: | ---: | ---: | ---: | ---: |",
            ]
        )
        for backend, group in sorted(allocation_groups.items()):
            metrics = group["metrics"]

            def allocation_value(name: str) -> int | float | None:
                result = metrics.get(name)
                return None if result is None else result["median"]

            lines.append(
                f"| {backend} | {group['samples']} | "
                f"{allocation_value('entry_allocation_calls') or '—'} | "
                f"{(float(allocation_value('entry_allocated_bytes')) / 1024 / 1024):.2f} | "
                f"{(float(allocation_value('final_live_bytes')) / 1024 / 1024):.2f} | "
                f"{(float(allocation_value('peak_live_bytes')) / 1024 / 1024):.2f} |"
            )
    else:
        lines.append("No allocation profile for this harness commit.")
    lines.append("")
    return "\n".join(lines)


def main() -> None:
    args = parse_args()
    records = [
        record
        for record in read_records(args.raw_jsonl)
        if record.get("harness_commit") == args.harness_commit
    ]
    timed = [
        record
        for record in records
        if record.get("status") == "ok"
        and isinstance(record.get("measurement"), dict)
        and "sample" in record
        and record.get("scenario") != "allocation-profile"
    ]
    failures = [
        record
        for record in records
        if record.get("status") != "ok"
        and record.get("scenario") not in {"full-snapshot-parity"}
    ]
    if failures:
        raise RuntimeError(
            f"{len(failures)} failed records exist for harness {args.harness_commit}"
        )

    grouped_records: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for record in timed:
        key = f"{record['backend']}|{record['cache_stance']}"
        grouped_records[key].append(record)
    groups = {
        key: summarize_group(sorted(value, key=lambda record: record["sample"]))
        for key, value in sorted(grouped_records.items())
    }
    add_relatives(groups)
    parity_records = [
        record
        for record in records
        if record.get("scenario") == "full-snapshot-parity"
    ]
    aggregate_records = [
        record
        for record in records
        if record.get("scenario") == "aggregate-four-reader-pss"
    ]
    allocation_records = [
        record
        for record in records
        if record.get("scenario") == "allocation-profile"
        and record.get("status") == "ok"
    ]
    grouped_allocations: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for record in allocation_records:
        grouped_allocations[record["backend"]].append(record)
    allocation_groups = {
        backend: summarize_allocation_group(group)
        for backend, group in sorted(grouped_allocations.items())
    }
    summary = {
        "schema": "hbk-snapshot-benchmark-summary-v1",
        "harness_commit": args.harness_commit,
        "ranked": False,
        "groups": groups,
        "parity": parity_records,
        "aggregate_four_reader": aggregate_records,
        "allocation_profiles": allocation_groups,
    }
    args.json_path.parent.mkdir(parents=True, exist_ok=True)
    args.markdown_path.parent.mkdir(parents=True, exist_ok=True)
    args.json_path.write_text(
        json.dumps(summary, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    args.markdown_path.write_text(
        render_markdown(
            args.harness_commit,
            groups,
            parity_records,
            aggregate_records,
            allocation_groups,
        ),
        encoding="utf-8",
    )


if __name__ == "__main__":
    main()
