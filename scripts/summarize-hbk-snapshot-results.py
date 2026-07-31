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
    "materialize_ns": ("measurement", "timings", "open", "elapsed_ns"),
    "artifact_write_ns": (
        "measurement",
        "timings",
        "cache_write",
        "elapsed_ns",
    ),
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

AGGREGATE_METRICS: dict[str, tuple[str, ...]] = {
    "rss_kib": ("aggregate", "rss_kib"),
    "pss_kib": ("aggregate", "pss_kib"),
    "private_kib": ("aggregate", "private_kib"),
    "shared_kib": ("aggregate", "shared_kib"),
    "anonymous_kib": ("aggregate", "anonymous_kib"),
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


def summarize_machine_state(
    records: list[dict[str, Any]],
) -> dict[str, int | float] | None:
    loads: list[float] = []
    normalized_loads: list[float] = []
    available_memory: list[int] = []
    for record in records:
        for key in (
            "machine_state_before",
            "machine_state_at_hold",
            "machine_state_after",
        ):
            state = record.get(key)
            if not isinstance(state, dict):
                continue
            load = nested(state, ("load_average", "one_minute"))
            logical_cpus = nested(state, ("logical_cpus",))
            memory = nested(state, ("memory", "available_kib"))
            if load is not None:
                loads.append(float(load))
                if logical_cpus is not None and logical_cpus > 0:
                    normalized_loads.append(float(load) / float(logical_cpus))
            if memory is not None:
                available_memory.append(int(memory))
    if not loads and not available_memory:
        return None
    result: dict[str, int | float] = {
        "snapshots": max(len(loads), len(available_memory)),
    }
    if loads:
        result["max_one_minute_load"] = max(loads)
    if normalized_loads:
        result["max_one_minute_load_per_logical_cpu"] = max(normalized_loads)
    if available_memory:
        result["min_available_memory_kib"] = min(available_memory)
    return result


GroupIdentity = tuple[str, str, str, str, str]


def group_identity(record: dict[str, Any]) -> GroupIdentity:
    return (
        str(record.get("dataset", "")),
        str(record.get("backend", "")),
        str(record.get("cache_stance", "")),
        str(record.get("candidate_branch", "")),
        str(record.get("candidate_commit", "")),
    )


def identity_json_key(identity: GroupIdentity) -> str:
    return json.dumps(identity, ensure_ascii=False, separators=(",", ":"))


def identity_dict(identity: GroupIdentity) -> dict[str, str]:
    dataset, backend, stance, branch, commit = identity
    return {
        "dataset": dataset,
        "backend": backend,
        "cache_stance": stance,
        "candidate_branch": branch,
        "candidate_commit": commit,
    }


def summarize_group(records: list[dict[str, Any]]) -> dict[str, Any]:
    identity = group_identity(records[0])
    if any(group_identity(record) != identity for record in records):
        raise ValueError("attempted to pool benchmark records with different identities")
    summary: dict[str, Any] = {
        "identity": identity_dict(identity),
        "samples": len(records),
        "sample_ids": [record["sample"] for record in records],
        "metrics": {},
        "operations": {},
    }
    machine_state = summarize_machine_state(records)
    if machine_state is not None:
        summary["machine_state"] = machine_state
    for name, path in METRICS.items():
        metric = median_mad(
            value for record in records if (value := nested(record, path)) is not None
        )
        if metric is not None:
            summary["metrics"][name] = metric
    resident_growth = []
    for record in records:
        before = record.get("resident_bytes_before")
        after = record.get("resident_bytes_after")
        if not isinstance(before, dict) or not isinstance(after, dict):
            continue
        resident_growth.append(
            sum(
                max(0, int(after.get(path, 0)) - int(bytes_before))
                for path, bytes_before in before.items()
            )
        )
    resident_metric = median_mad(resident_growth)
    if resident_metric is not None:
        summary["metrics"]["file_resident_growth_bytes"] = resident_metric

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
    identity = group_identity(records[0])
    if any(group_identity(record) != identity for record in records):
        raise ValueError("attempted to pool allocation records with different identities")
    summary: dict[str, Any] = {
        "identity": identity_dict(identity),
        "samples": len(records),
        "metrics": {},
    }
    machine_state = summarize_machine_state(records)
    if machine_state is not None:
        summary["machine_state"] = machine_state
    for name, path in ALLOCATION_METRICS.items():
        metric = median_mad(
            value for record in records if (value := nested(record, path)) is not None
        )
        if metric is not None:
            summary["metrics"][name] = metric
    return summary


def summarize_aggregate_group(records: list[dict[str, Any]]) -> dict[str, Any]:
    identity = group_identity(records[0])
    if any(group_identity(record) != identity for record in records):
        raise ValueError("attempted to pool aggregate records with different identities")
    summary: dict[str, Any] = {
        "identity": identity_dict(identity),
        "samples": len(records),
        "metrics": {},
    }
    machine_state = summarize_machine_state(records)
    if machine_state is not None:
        summary["machine_state"] = machine_state
    for name, path in AGGREGATE_METRICS.items():
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
    baselines: dict[tuple[str, str, str], dict[str, Any]] = {}
    for group in groups.values():
        identity = group["identity"]
        backend = identity["backend"]
        if backend in {"sql-owned", "cache-owned", "cache-owned-produce"}:
            baselines[
                (identity["dataset"], identity["cache_stance"], backend)
            ] = group

    for group in groups.values():
        identity = group["identity"]
        dataset = identity["dataset"]
        stance = identity["cache_stance"]
        backend = identity["backend"]
        comparator_names = (
            ("cache", "cache-owned-produce"),
        ) if backend.endswith("-produce") else (
            ("sql", "sql-owned"),
            ("cache", "cache-owned"),
        )
        for label, comparator_name in comparator_names:
            baseline = baselines.get((dataset, stance, comparator_name))
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
            group[f"relative_to_{label}_percent"] = relatives


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
    aggregate_groups: dict[str, dict[str, Any]],
    allocation_groups: dict[str, dict[str, Any]],
) -> str:
    lines = [
        "# HBK Snapshot Comparison (Unranked)",
        "",
        f"Frozen harness commit: `{harness_commit}`.",
        "",
        "Rows are evidence only. This report does not assign a score, rank or winner.",
        "",
        "| Backend | Branch | Commit | Cache stance | N | Ready ms | First lookup µs | Workload ms | Peak RSS MiB | Post-workload PSS MiB | Post-workload private MiB | Open minor faults |",
        "| --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
    ]
    sorted_groups = sorted(
        groups.values(),
        key=lambda group: tuple(group["identity"].values()),
    )
    for group in sorted_groups:
        identity = group["identity"]
        lines.append(
            "| "
            + " | ".join(
                [
                    identity["backend"],
                    identity["candidate_branch"] or "—",
                    identity["candidate_commit"][:12] or "—",
                    identity["cache_stance"],
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

    production_groups = [
        group
        for group in sorted_groups
        if group["identity"]["backend"].endswith("-produce")
    ]
    lines.extend(["", "## Artifact production", ""])
    if production_groups:
        lines.extend(
            [
                "| Backend | Branch | Commit | N | Total local rebuild ms | Materialize ms | Artifact write ms | Artifact MiB | Peak RSS MiB |",
                "| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |",
            ]
        )
        for group in production_groups:
            identity = group["identity"]
            artifact_bytes = metric_median(group, "artifact_bytes")
            lines.append(
                f"| {identity['backend']} | "
                f"{identity['candidate_branch'] or '—'} | "
                f"{identity['candidate_commit'][:12] or '—'} | "
                f"{group['samples']} | "
                f"{format_ms(metric_median(group, 'ready_ns'))} | "
                f"{format_ms(metric_median(group, 'materialize_ns'))} | "
                f"{format_ms(metric_median(group, 'artifact_write_ns'))} | "
                f"{'—' if artifact_bytes is None else f'{float(artifact_bytes) / 1024 / 1024:.2f}'} | "
                f"{format_mib(metric_median(group, 'peak_rss_kib'))} |"
            )
    else:
        lines.append("No artifact-production record for this harness commit.")

    lines.extend(["", "## Parity evidence", ""])
    if parity_records:
        lines.extend(
            [
                "| Backend | Branch | Commit | Status | Content SHA-256 | Lookup SHA-256 |",
                "| --- | --- | --- | --- | --- | --- |",
            ]
        )
        for record in parity_records:
            lines.append(
                f"| {record.get('backend', '—')} | "
                f"{record.get('candidate_branch') or '—'} | "
                f"{str(record.get('candidate_commit', ''))[:12] or '—'} | "
                f"{record.get('status', '—')} | "
                f"`{record.get('content_sha256', '—')}` | "
                f"`{record.get('lookup_sha256', '—')}` |"
            )
    else:
        lines.append("No parity record for this harness commit.")

    lines.extend(["", "## Aggregate four-reader PSS", ""])
    if aggregate_groups:
        lines.extend(
            [
                "| Backend | Branch | Commit | N | Aggregate PSS MiB (median ± MAD) | Aggregate private MiB (median ± MAD) |",
                "| --- | --- | --- | ---: | ---: | ---: |",
            ]
        )
        for group in sorted(
            aggregate_groups.values(),
            key=lambda value: tuple(value["identity"].values()),
        ):
            identity = group["identity"]
            pss = group["metrics"]["pss_kib"]
            private = group["metrics"]["private_kib"]
            lines.append(
                f"| {identity['backend']} | "
                f"{identity['candidate_branch'] or '—'} | "
                f"{identity['candidate_commit'][:12] or '—'} | "
                f"{group['samples']} | "
                f"{float(pss['median']) / 1024:.2f} ± "
                f"{float(pss['mad']) / 1024:.2f} | "
                f"{float(private['median']) / 1024:.2f} ± "
                f"{float(private['mad']) / 1024:.2f} |"
            )
    else:
        lines.append("No aggregate-reader record for this harness commit.")

    lines.extend(["", "## Allocation profiles", ""])
    if allocation_groups:
        lines.extend(
            [
                "| Backend | Branch | Commit | N | Entry allocations | Entry allocated MiB | Final live MiB | Peak live MiB |",
                "| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |",
            ]
        )
        for group in sorted(
            allocation_groups.values(),
            key=lambda value: tuple(value["identity"].values()),
        ):
            identity = group["identity"]
            metrics = group["metrics"]

            def allocation_value(name: str) -> int | float | None:
                result = metrics.get(name)
                return None if result is None else result["median"]

            lines.append(
                f"| {identity['backend']} | "
                f"{identity['candidate_branch'] or '—'} | "
                f"{identity['candidate_commit'][:12] or '—'} | "
                f"{group['samples']} | "
                f"{allocation_value('entry_allocation_calls') or '—'} | "
                f"{(float(allocation_value('entry_allocated_bytes')) / 1024 / 1024):.2f} | "
                f"{(float(allocation_value('final_live_bytes')) / 1024 / 1024):.2f} | "
                f"{(float(allocation_value('peak_live_bytes')) / 1024 / 1024):.2f} |"
            )
    else:
        lines.append("No allocation profile for this harness commit.")

    lines.extend(["", "## Host pressure evidence", ""])
    pressure_rows = [
        ("runtime", group)
        for group in sorted_groups
        if "machine_state" in group
    ]
    pressure_rows.extend(
        ("four-reader", group)
        for group in sorted(
            aggregate_groups.values(),
            key=lambda value: tuple(value["identity"].values()),
        )
        if "machine_state" in group
    )
    pressure_rows.extend(
        ("allocation", group)
        for group in sorted(
            allocation_groups.values(),
            key=lambda value: tuple(value["identity"].values()),
        )
        if "machine_state" in group
    )
    if pressure_rows:
        lines.extend(
            [
                "| Scenario | Backend | Branch | Samples | State snapshots | Max load1 / logical CPU | Min available memory GiB |",
                "| --- | --- | --- | ---: | ---: | ---: | ---: |",
            ]
        )
        for scenario, group in pressure_rows:
            identity = group["identity"]
            state = group["machine_state"]
            normalized_load = state.get("max_one_minute_load_per_logical_cpu")
            available_kib = state.get("min_available_memory_kib")
            lines.append(
                f"| {scenario} | "
                f"{identity['backend']} | "
                f"{identity['candidate_branch'] or '—'} | "
                f"{group['samples']} | "
                f"{state['snapshots']} | "
                f"{'—' if normalized_load is None else f'{float(normalized_load):.4f}'} | "
                f"{'—' if available_kib is None else f'{float(available_kib) / 1024 / 1024:.2f}'} |"
            )
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

    grouped_records: dict[GroupIdentity, list[dict[str, Any]]] = defaultdict(list)
    for record in timed:
        grouped_records[group_identity(record)].append(record)
    groups = {
        identity_json_key(identity): summarize_group(
            sorted(value, key=lambda record: record["sample"])
        )
        for identity, value in sorted(grouped_records.items())
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
    grouped_aggregates: dict[GroupIdentity, list[dict[str, Any]]] = defaultdict(list)
    for record in aggregate_records:
        grouped_aggregates[group_identity(record)].append(record)
    aggregate_groups = {
        identity_json_key(identity): summarize_aggregate_group(group)
        for identity, group in sorted(grouped_aggregates.items())
    }
    allocation_records = [
        record
        for record in records
        if record.get("scenario") == "allocation-profile"
        and record.get("status") == "ok"
    ]
    grouped_allocations: dict[GroupIdentity, list[dict[str, Any]]] = defaultdict(list)
    for record in allocation_records:
        grouped_allocations[group_identity(record)].append(record)
    allocation_groups = {
        identity_json_key(identity): summarize_allocation_group(group)
        for identity, group in sorted(grouped_allocations.items())
    }
    summary = {
        "schema": "hbk-snapshot-benchmark-summary-v1",
        "harness_commit": args.harness_commit,
        "ranked": False,
        "groups": groups,
        "parity": parity_records,
        "aggregate_four_reader": aggregate_groups,
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
            aggregate_groups,
            allocation_groups,
        ),
        encoding="utf-8",
    )


if __name__ == "__main__":
    main()
