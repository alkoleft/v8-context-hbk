#!/usr/bin/env python3

from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path
from typing import Any


SCRIPTS = Path(__file__).resolve().parents[1]


def load_module(name: str, path: Path) -> Any:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


benchmark = load_module("hbk_s83_av2_benchmark", SCRIPTS / "benchmark-hbk-s83-av2.py")
summarizer = load_module("hbk_s83_av2_summarizer", SCRIPTS / "summarize-hbk-s83-av2-results.py")


def backend(name: str = "S83-H0") -> Any:
    return benchmark.Backend(
        backend=name,
        decision_role=benchmark.DECISION_ROLES[name],
        worktree=Path("/tmp/example"),
        command=("/bin/true", "{mode}", "{operation}", "{context}", "{iterations}", "{query_manifest}"),
        declared_files=(Path("/tmp/provider"),),
        declared_file_artifacts=(
            {"path": "/tmp/provider", "bytes": benchmark.PROVIDER_BYTES, "sha256": benchmark.PROVIDER_SHA256},
        ),
        executable=Path("/bin/true"),
        executable_artifact={"path": "/bin/true", "bytes": 1, "sha256": "0" * 64},
        commit="a" * 40,
        branch="experiment/av2",
    )


def artifact(path: str = "/tmp/provider") -> dict[str, Any]:
    return {"path": path, "bytes": benchmark.PROVIDER_BYTES, "sha256": benchmark.PROVIDER_SHA256}


def allocation_delta(value: int = 1) -> dict[str, int]:
    return {
        "allocation_calls": value,
        "reallocation_calls": 0,
        "deallocation_calls": 0,
        "allocated_bytes": value,
        "deallocated_bytes": 0,
        "live_bytes_before": 0,
        "live_bytes_after": value,
        "peak_live_bytes_before": 0,
        "peak_live_bytes_after": value,
        "peak_live_bytes_growth": value,
    }


def timing_phase(value: int = 10) -> dict[str, Any]:
    return {
        "elapsed_ns": value,
        "average_ns": value,
        "ns_per_query": value,
        "ns_per_object": value,
        "count": 1,
        "checksum": value,
    }


def memory(operation: str) -> dict[str, Any]:
    sample = {"rss_kib": 1, "pss_kib": 1, "private_kib": 1, "anonymous_kib": 1, "file_backed_kib": 0}
    compact = operation == "members_by_owner_availability_collect"
    return {
        "before_kib": dict(sample),
        "live_kib": dict(sample),
        "after_drop_kib": dict(sample),
        "container_overhead_bytes": 16 if compact else 0,
        "logical_bytes": 8 if compact else 0,
        "capacity_bytes": 12 if compact else 0,
        "live_delta_bytes": 12 if compact else 0,
        "peak_live_delta_bytes": 12 if compact else 0,
        "post_drop_delta_bytes": 0,
    }


def operation_data(operation: str) -> dict[str, Any]:
    if operation in benchmark.LOOKUP_OPERATIONS:
        return {"tag": "lookup", "query_count": 3, "candidate_count": 2, "miss_count": 1}
    if operation == "members_by_owner_availability_borrowed":
        return {
            "tag": "iteration",
            "owner_count": 1,
            "scanned_count": 3,
            "returned_count": 2,
            "universal_count": 1,
            "explicit_count": 1,
            "excluded_count": 1,
            "property_count": 1,
            "method_count": 1,
            "event_count": 0,
            "enum_value_count": 0,
        }
    if operation == "members_by_owner_availability_collect":
        return {
            "tag": "compact_materialization",
            "owner_count": 1,
            "scanned_count": 3,
            "returned_count": 2,
            "universal_count": 1,
            "explicit_count": 1,
            "excluded_count": 1,
            "property_count": 1,
            "method_count": 1,
            "event_count": 0,
            "enum_value_count": 0,
            "locator_size": 4,
            "total_len": 2,
            "total_capacity": 3,
            "logical_bytes": 8,
            "allocated_bytes": 12,
        }
    return {
        "tag": "payload",
        "input_count": 2,
        "object_count": 2,
        "string_bytes_touched": 4,
        "canonical_payload_bytes_touched": 8,
    }


def report(name: str = "S83-H0", operation: str = "type_by_name", context: str | None = None, iterations: int = 100, parity: bool = False) -> dict[str, Any]:
    phase_map = {phase: timing_phase(10) for phase in benchmark.PHASE_ORDER}
    allocation_map: dict[str, Any] = {"enabled": True}
    allocation_map.update({phase: allocation_delta(1) for phase in benchmark.PHASE_ORDER})
    value: dict[str, Any] = {
        "schema_version": benchmark.REPORT_SCHEMA,
        "workload_version": benchmark.WORKLOAD_VERSION,
        "mode": "performance",
        "backend": name,
        "decision_role": benchmark.DECISION_ROLES[name],
        "operation": operation,
        "availability_context": context,
        "iterations": iterations,
        "module_context_filter_used": False,
        "empty_availability_rule": "universal",
        "input_identity": {
            "dataset": benchmark.DATASET,
            "platform_version": benchmark.PLATFORM_VERSION,
            "source_locale": benchmark.SOURCE_LOCALE,
            "provider_schema_version": benchmark.PROVIDER_SCHEMA_VERSION,
            "extraction_schema_version": benchmark.EXTRACTION_SCHEMA_VERSION,
            "hbk": {"path": "/tmp/hbk", "bytes": benchmark.HBK_BYTES, "sha256": benchmark.HBK_SHA256},
            "provider": artifact(),
        },
        "manifest": {"schema_version": benchmark.MANIFEST_SCHEMA, "sha256": "d" * 64, "bytes": 100},
        "runtime_artifacts": [artifact()],
        "projection": dict(benchmark.PROJECTION_REGISTRY[name][operation]),
        "phase_order": list(benchmark.PHASE_ORDER),
        "timings": phase_map,
        "faults": {phase: {"minor": 0, "major": 0} for phase in benchmark.PHASE_ORDER},
        "allocations": allocation_map,
        "memory": memory(operation),
        "counts": {
            "query_count": 1,
            "candidate_count": 1,
            "object_count": 1,
            "checksum_count": 1,
            "property_count": 1,
            "method_count": 0,
            "event_count": 0,
            "enum_value_count": 0,
        },
        "checksum": {"value": 42, "algorithm": "rolling-u64"},
        "operation_data": operation_data(operation),
    }
    if parity:
        value["parity_transcript"] = [{"id": operation, "context": context}]
    return value


def parity_report(name: str = "S83-H0", context: str = "thin_client") -> dict[str, Any]:
    return {
        "schema_version": benchmark.PARITY_SCHEMA,
        "workload_version": benchmark.WORKLOAD_VERSION,
        "mode": "parity",
        "backend": name,
        "decision_role": benchmark.DECISION_ROLES[name],
        "availability_context": context,
        "module_context_filter_used": False,
        "empty_availability_rule": "universal",
        "input_identity": {
            "dataset": benchmark.DATASET,
            "platform_version": benchmark.PLATFORM_VERSION,
            "source_locale": benchmark.SOURCE_LOCALE,
            "provider_schema_version": benchmark.PROVIDER_SCHEMA_VERSION,
            "extraction_schema_version": benchmark.EXTRACTION_SCHEMA_VERSION,
            "hbk": {"path": "/tmp/hbk", "bytes": benchmark.HBK_BYTES, "sha256": benchmark.HBK_SHA256},
            "provider": artifact(),
        },
        "manifest": {"schema_version": benchmark.MANIFEST_SCHEMA, "sha256": "d" * 64, "bytes": 100},
        "runtime_artifacts": [artifact()],
        "transcript": {"context": context, "records": [{"id": "all_operations"}]},
    }


def query_manifest() -> dict[str, Any]:
    return {
        "schema_version": benchmark.MANIFEST_SCHEMA,
        "workload_version": benchmark.WORKLOAD_VERSION,
        "input_identity": {
            "dataset": benchmark.DATASET,
            "platform_version": benchmark.PLATFORM_VERSION,
            "source_locale": benchmark.SOURCE_LOCALE,
            "provider_schema_version": benchmark.PROVIDER_SCHEMA_VERSION,
            "extraction_schema_version": benchmark.EXTRACTION_SCHEMA_VERSION,
            "hbk": {"path": "/tmp/hbk", "bytes": benchmark.HBK_BYTES, "sha256": benchmark.HBK_SHA256},
            "provider": artifact(),
        },
        "availability_contexts": list(benchmark.AVAILABILITY_CONTEXTS),
        "member_kinds": ["property", "method", "event", "enum_value"],
        "empty_availability_rule": "universal",
        "module_context_filter_used": False,
        "types": [
            {"logical_id": "platform_type:Запрос", "primary": "Запрос", "alias": None, "member_count": 1},
        ],
        "members": [
            {
                "logical_id": "platform_type:Запрос/member:property:Текст",
                "owner_logical_id": "platform_type:Запрос",
                "kind": "property",
                "primary": "Текст",
                "alias": None,
            },
        ],
        "lookup_queries": {
            "type_names": [
                {
                    "logical_id": "platform_type:Запрос",
                    "query_name": "Запрос",
                    "query_role": "primary",
                },
            ],
            "properties": [
                {
                    "logical_id": "platform_type:Запрос/member:property:Текст",
                    "owner_logical_id": "platform_type:Запрос",
                    "kind": "property",
                    "query_name": "Текст",
                    "query_role": "primary",
                },
            ],
            "methods": [],
        },
        "fixed_misses": {
            "type_name": "__hbk_s83_av2_missing_type__",
            "member_name": "__hbk_s83_av2_missing_member__",
            "callable_name": "__hbk_s83_av2_missing_callable__",
        },
        "anchors": {
            "type_primary": "Запрос",
            "property_owner": "platform_type:Запрос",
            "property_name": "Текст",
            "method_owner": "platform_type:Запрос",
            "method_name": "Выполнить",
            "enumeration_owner": "platform_type:ФормаКлиентскогоПриложения",
        },
    }


def raw_record(name: str, operation: str, context: str | None, stance: str, sample: int, multiplier: int = 1) -> dict[str, Any]:
    measurement = report(name, operation, context, iterations=10, parity=False)
    measurement["timings"]["steady_workload"]["elapsed_ns"] = 100 * multiplier + sample
    measurement["timings"]["steady_workload"]["average_ns"] = 10 * multiplier + sample
    return {
        "schema": benchmark.RAW_SCHEMA,
        "dataset": benchmark.DATASET,
        "platform_version": benchmark.PLATFORM_VERSION,
        "provider_schema_version": benchmark.PROVIDER_SCHEMA_VERSION,
        "extraction_schema_version": benchmark.EXTRACTION_SCHEMA_VERSION,
        "hbk_sha256": benchmark.HBK_SHA256,
        "provider_sha256": benchmark.PROVIDER_SHA256,
        "backend": name,
        "decision_role": benchmark.DECISION_ROLES[name],
        "candidate_commit": "a" * 40,
        "candidate_branch": "experiment/av2",
        "worktree": "/tmp/example",
        "executable_artifact": {"path": "/bin/true", "bytes": 1, "sha256": "0" * 64},
        "harness_commit": "b" * 40,
        "harness_branch": "main",
        "harness_file_sha256": {"scripts/benchmark-hbk-s83-av2.py": "c" * 64},
        "manifest_sha256": "d" * 64,
        "manifest_bytes": 100,
        "host": {"hostname": "test"},
        "orchestration_version": benchmark.ORCHESTRATION_VERSION,
        "backend_registry": list(benchmark.BACKENDS),
        "operation_registry": list(benchmark.OPERATIONS),
        "availability_context_registry": list(benchmark.AVAILABILITY_CONTEXTS),
        "cache_stance_registry": list(benchmark.CACHE_STANCES),
        "planned_samples_per_row": 1,
        "declared_file_artifacts": [artifact()],
        "operation": operation,
        "availability_context": context,
        "cache_stance": stance,
        "sample": sample,
        "iterations": 10,
        "status": "ok",
        "command_template": ["/bin/true", "{mode}", "{operation}", "{context}", "{iterations}", "{query_manifest}"],
        "command": ["/bin/true", "performance", operation, *( [] if context is None else [context] ), "10", "/tmp/query-manifest.json"],
        "declared_files": ["/tmp/provider"],
        "preparation": {"method": stance, "declared_files": ["/tmp/provider"]},
        "machine_state_before": {"load": 0},
        "machine_state_after": {"load": 0},
        "stdout_log": "/tmp/out",
        "stderr_log": "/tmp/err",
        "stderr_sha256": "e" * 64,
        "h0_parity_sha256": "f" * 64,
        "measurement": measurement,
    }


class BenchmarkAv2ContractTests(unittest.TestCase):
    def test_run_parameters_are_frozen_for_v1(self) -> None:
        args = benchmark.parse_args(
            ["--manifest", "/tmp/orchestration.json", "--results-root", "/tmp/results"]
        )
        benchmark.validate_frozen_run_parameters(args)
        args.samples = benchmark.DEFAULT_SAMPLES - 1
        with self.assertRaisesRegex(benchmark.EvidenceError, "samples must remain frozen"):
            benchmark.validate_frozen_run_parameters(args)

    def test_validates_standalone_parity_transcript_bytes(self) -> None:
        transcript, stripped = benchmark.validate_parity_report(parity_report(), backend(), "thin_client", "d" * 64, 100)
        self.assertEqual(transcript, benchmark.canonical_json_bytes(stripped["transcript"]))
        self.assertNotIn("operation", stripped)

    def test_rejects_parity_as_benchmark_report(self) -> None:
        value = report(parity=True)
        with self.assertRaisesRegex(benchmark.EvidenceError, "schema keys"):
            benchmark.validate_report(value, backend(), "type_by_name", None, 100, "d" * 64, 100)

    def test_command_template_drops_empty_parity_args_and_keeps_query_manifest(self) -> None:
        command = benchmark.command_for(
            backend().command,
            "parity",
            None,
            "thin_client",
            None,
            "/tmp/query-manifest.json",
        )
        self.assertEqual(command, ["/bin/true", "parity", "thin_client", "/tmp/query-manifest.json"])

    def test_rejects_unknown_nested_module_context_kind(self) -> None:
        value = report()
        value["operation_data"]["nested_ModuleContextKind"] = "Client"
        with self.assertRaisesRegex(benchmark.EvidenceError, "ModuleContextKind"):
            benchmark.validate_report(value, backend(), "type_by_name", None, 100, "d" * 64, 100)

    def test_rejects_non_performance_report_mode(self) -> None:
        value = report()
        value["mode"] = "sql-owned"
        with self.assertRaisesRegex(benchmark.EvidenceError, "mode expected 'performance'"):
            benchmark.validate_report(value, backend(), "type_by_name", None, 100, "d" * 64, 100)

    def test_validates_query_manifest_without_orchestration_registries(self) -> None:
        manifest = query_manifest()
        benchmark.validate_query_manifest_identity(manifest)
        manifest["backend_registry"] = list(benchmark.BACKENDS)
        with self.assertRaisesRegex(benchmark.EvidenceError, "schema keys"):
            benchmark.require_exact_keys(manifest, benchmark.QUERY_MANIFEST_KEYS, "query_manifest")

    def test_query_manifest_rejects_unknown_fields_and_nested_module_context_kind(self) -> None:
        manifest = query_manifest()
        manifest["types"][0]["unexpected"] = True
        with self.assertRaisesRegex(benchmark.EvidenceError, "unexpected"):
            benchmark.validate_query_manifest_identity(manifest)
        manifest = query_manifest()
        manifest["lookup_queries"]["properties"][0]["nested_ModuleContextKind"] = "Client"
        with self.assertRaisesRegex(benchmark.EvidenceError, "ModuleContextKind"):
            benchmark.reject_forbidden_fields(manifest)

    def test_query_manifest_rejects_incomplete_ordered_lookup_corpus(self) -> None:
        manifest = query_manifest()
        manifest["lookup_queries"]["type_names"] = []
        with self.assertRaisesRegex(benchmark.EvidenceError, "primary/distinct-alias"):
            benchmark.validate_query_manifest_identity(manifest)

    def test_rejects_wrong_projection_registry(self) -> None:
        value = report("S83-R1", "method_payload")
        value["projection"]["source"] = "decoded-value"
        with self.assertRaisesRegex(benchmark.EvidenceError, "projection"):
            benchmark.validate_report(value, backend("S83-R1"), "method_payload", None, 100, "d" * 64, 100)

    def test_rejects_compact_fields_on_lookup(self) -> None:
        value = report()
        value["operation_data"]["locator_size"] = 4
        with self.assertRaisesRegex(benchmark.EvidenceError, "schema keys"):
            benchmark.validate_report(value, backend(), "type_by_name", None, 100, "d" * 64, 100)

    def test_validates_compact_u32_locator_constraints(self) -> None:
        value = report(operation="members_by_owner_availability_collect", context="server", iterations=1000)
        benchmark.validate_report(value, backend(), "members_by_owner_availability_collect", "server", 1000, "d" * 64, 100)
        value["operation_data"]["allocated_bytes"] = 8
        with self.assertRaisesRegex(benchmark.EvidenceError, "allocated_bytes"):
            benchmark.validate_report(value, backend(), "members_by_owner_availability_collect", "server", 1000, "d" * 64, 100)

    def test_rejects_reordered_phase_order(self) -> None:
        value = report()
        value["phase_order"] = list(reversed(benchmark.PHASE_ORDER))
        with self.assertRaisesRegex(benchmark.EvidenceError, "phase_order"):
            benchmark.validate_report(value, backend(), "type_by_name", None, 100, "d" * 64, 100)

    def test_parity_plan_runs_all_h0_records_first(self) -> None:
        backends = [backend(name) for name in benchmark.BACKENDS]
        plan = list(benchmark.parity_plan(backends))
        h0_count = len(benchmark.AVAILABILITY_CONTEXTS)
        self.assertEqual(len(plan), 81)
        self.assertTrue(all(item[0].backend == "S83-H0" for item in plan[:h0_count]))
        self.assertTrue(all(item[0].backend != "S83-H0" for item in plan[h0_count:]))
        self.assertEqual([item[1] for item in plan[:h0_count]], list(benchmark.AVAILABILITY_CONTEXTS))

    def test_parity_limit_is_aggregate_per_backend(self) -> None:
        totals = {"S83-H0": 0}
        benchmark.add_parity_bytes(totals, "S83-H0", benchmark.MAX_PARITY_BYTES - 1)
        with self.assertRaisesRegex(benchmark.EvidenceError, "all availability contexts"):
            benchmark.add_parity_bytes(totals, "S83-H0", 2)

    def test_measurement_plan_round_robins_backends_inside_context_stance(self) -> None:
        backends = [backend(name) for name in benchmark.BACKENDS]
        plan = list(benchmark.measurement_plan(backends, 1))
        first_cell = plan[: len(benchmark.BACKENDS)]
        self.assertEqual([item[0].backend for item in first_cell], list(benchmark.BACKENDS))
        self.assertEqual({item[4] for item in first_cell}, {1})

    def test_orchestration_preflight_rejects_threshold_excess(self) -> None:
        manifest = {
            "manifest_bytes": benchmark.MAX_MANIFEST_BYTES + 1,
            "max_parity_bytes": 1,
            "raw_run_bytes": 1,
            "operation_context_rows": benchmark.OPERATION_CONTEXT_ROW_COUNT,
            "parity_rows": benchmark.PARITY_ROW_COUNT,
            "performance_rows": benchmark.PERFORMANCE_ROW_COUNT,
        }
        with self.assertRaisesRegex(benchmark.EvidenceError, "64 MiB"):
            benchmark.validate_orchestration_preflight(manifest, benchmark.MAX_MANIFEST_BYTES + 1)

    def test_measurement_requires_completed_parity(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            with self.assertRaisesRegex(benchmark.EvidenceError, "81 parity"):
                benchmark.measurement_phase(
                    [backend()],
                    Path(tmp),
                    {},
                    set(),
                    1,
                    1,
                    1,
                    1,
                    Path("/tmp/query-manifest.json"),
                    1,
                    {"S83-H0": {}},
                    "d" * 64,
                    100,
                )

    def test_measurement_requires_completed_smoke_after_parity(self) -> None:
        baseline = {context: b"ok" for context in benchmark.AVAILABILITY_CONTEXTS}
        with tempfile.TemporaryDirectory() as tmp:
            with self.assertRaisesRegex(benchmark.EvidenceError, "9 preflight smoke"):
                benchmark.measurement_phase(
                    [backend()],
                    Path(tmp),
                    baseline,
                    set(),
                    1,
                    1,
                    1,
                    1,
                    Path("/tmp/query-manifest.json"),
                    1,
                    {"S83-H0": {}},
                    "d" * 64,
                    100,
                )

    def test_smoke_plan_cardinality_is_frozen(self) -> None:
        backends = [backend(name) for name in benchmark.BACKENDS]
        plan = list(benchmark.smoke_plan(backends))
        self.assertEqual(len(plan), 9)
        self.assertEqual([item.backend for item in plan], list(benchmark.BACKENDS))

    def test_results_root_must_be_empty(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "existing").write_text("x", encoding="utf-8")
            with self.assertRaisesRegex(benchmark.EvidenceError, "empty"):
                benchmark.verify_results_root_empty(root)


class SummarizerAv2Tests(unittest.TestCase):
    def test_cli_expected_samples_are_frozen_for_v1(self) -> None:
        summarizer.validate_expected_samples(benchmark.DEFAULT_SAMPLES)
        with self.assertRaisesRegex(summarizer.SummaryError, "must remain frozen"):
            summarizer.validate_expected_samples(1)

    def complete_records(self) -> list[dict[str, Any]]:
        records = []
        for backend_name in benchmark.BACKENDS:
            multiplier = 1 if backend_name == "S83-H0" else 2
            for operation in benchmark.OPERATIONS:
                for context in benchmark.operation_contexts(operation):
                    for stance in benchmark.CACHE_STANCES:
                        records.append(raw_record(backend_name, operation, context, stance, 1, multiplier))
        return records

    def test_summary_requires_full_matrix_and_h0_ratios(self) -> None:
        summary = summarizer.build_summary(self.complete_records(), 1)
        self.assertEqual(summary["schema"], benchmark.SUMMARY_SCHEMA)
        h0_row = next(row for row in summary["rows"] if row["backend"] == "S83-H0")
        c0_row = next(row for row in summary["rows"] if row["backend"] == "S83-C0")
        self.assertEqual(h0_row["metrics"]["steady_workload_ns"]["h0_ratio"], 1.0)
        self.assertGreater(c0_row["metrics"]["steady_workload_ns"]["h0_ratio"], 1.0)

    def test_default_cardinality_is_frozen(self) -> None:
        self.assertEqual(benchmark.OPERATION_CONTEXT_ROW_COUNT, 34)
        self.assertEqual(benchmark.PARITY_ROW_COUNT, 81)
        self.assertEqual(benchmark.PERFORMANCE_ROW_COUNT, 5508)

    def test_summary_rejects_missing_matrix_row(self) -> None:
        records = self.complete_records()
        records.pop()
        with self.assertRaisesRegex(summarizer.SummaryError, "incomplete"):
            summarizer.build_summary(records, 1)

    def test_summary_rejects_duplicate_matrix_row(self) -> None:
        records = self.complete_records()
        records.append(dict(records[0]))
        with self.assertRaisesRegex(summarizer.SummaryError, "duplicate"):
            summarizer.build_summary(records, 1)

    def test_summary_rejects_ranking_fields(self) -> None:
        records = self.complete_records()
        records[0]["winner"] = True
        with self.assertRaisesRegex(summarizer.contract.EvidenceError, "ranking"):
            summarizer.build_summary(records, 1)

    def test_summary_rejects_canonical_transcript_inside_raw_measurement(self) -> None:
        records = self.complete_records()
        records[0]["measurement"]["canonical_transcript"] = []
        with self.assertRaisesRegex((summarizer.SummaryError, summarizer.contract.EvidenceError), "canonical"):
            summarizer.build_summary(records, 1)


if __name__ == "__main__":
    unittest.main()
