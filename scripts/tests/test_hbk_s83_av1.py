#!/usr/bin/env python3

from __future__ import annotations

import hashlib
import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path
from typing import Any
from unittest import mock


SCRIPTS = Path(__file__).resolve().parents[1]


def load_module(name: str, path: Path) -> Any:
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


benchmark = load_module("hbk_s83_av1_benchmark", SCRIPTS / "benchmark-hbk-s83-av1.py")
summarizer = load_module(
    "hbk_s83_av1_summarizer", SCRIPTS / "summarize-hbk-s83-av1-results.py"
)


def backend(name: str = "S83-H0") -> Any:
    role = "baseline" if name == "S83-H0" else "control" if name == "S83-C0" else "candidate"
    return benchmark.Backend(
        backend=name,
        decision_role=role,
        worktree=Path("/tmp/example"),
        command=("probe", "{context}", "{iterations}"),
        declared_files=(Path("/tmp/input"),),
        commit="a" * 40,
        branch="experiment/example",
    )


def probe_report(
    name: str = "S83-H0", context: str = "server", iterations: int = 3
) -> dict[str, Any]:
    role = "baseline" if name == "S83-H0" else "control" if name == "S83-C0" else "candidate"
    allocation = {"allocation_calls": 1, "allocated_bytes": 8}
    return {
        "schema_version": benchmark.REPORT_SCHEMA,
        "workload_version": benchmark.WORKLOAD_VERSION,
        "mode": "test",
        "backend": name,
        "decision_role": role,
        "baseline_role": "h0",
        "module_context_filter_used": False,
        "empty_availability_rule": "universal",
        "availability_context": context,
        "iterations": iterations,
        "input_identity": {
            "platform_version": benchmark.PLATFORM_VERSION,
            "source_locale": benchmark.SOURCE_LOCALE,
            "provider_schema_version": benchmark.PROVIDER_SCHEMA_VERSION,
            "extraction_schema_version": benchmark.EXTRACTION_SCHEMA_VERSION,
            "hbk": {
                "path": "/hbk",
                "bytes": benchmark.HBK_BYTES,
                "sha256": benchmark.HBK_SHA256,
            },
            "provider": {
                "path": "/provider",
                "bytes": benchmark.PROVIDER_BYTES,
                "sha256": benchmark.PROVIDER_SHA256,
            },
        },
        "counts": {
            "scanned_globals": 4,
            "candidate_methods": 3,
            "returned_objects": 2,
            "universal_objects": 1,
            "explicit_context_objects": 1,
            "excluded_objects": 1,
            "universal_assertion": True,
            "excluded_assertion": True,
        },
        "timings": {
            "phase_order": ["entry_to_ready", "first_enumeration", "warmup", "workload"],
            "entry_to_ready_ns": 10,
            "first_enumeration": {
                "elapsed_ns": 20,
                "ns_per_object": 10,
                "faults": {"minor": 1, "major": 0},
                "returned_objects": 2,
                "checksum": 1,
            },
            "warmup": {
                "elapsed_ns": 15,
                "ns_per_object": 7,
                "faults": {"minor": 0, "major": 0},
                "returned_objects": 2,
                "checksum": 1,
            },
            "workload": {
                "elapsed_ns": 30,
                "average_ns": 10,
                "ns_per_object": 5,
                "faults": {"minor": 0, "major": 0},
                "iterations": iterations,
                "returned_total": iterations * 2,
                "checksum": 1,
            },
        },
        "allocations": {
            "enabled": True,
            "first_enumeration": dict(allocation),
            "workload": dict(allocation),
        },
        "transcript": [{"id": "universal"}, {"id": "explicit"}],
    }


def digest_for(context: str) -> str:
    return hashlib.sha256(context.encode()).hexdigest()


def raw_record(
    name: str,
    context: str,
    stance: str,
    sample: int,
    multiplier: int,
) -> dict[str, Any]:
    role = "baseline" if name == "S83-H0" else "control" if name == "S83-C0" else "candidate"
    digest = digest_for(context)
    value = multiplier * (100 + sample)
    return {
        "schema": summarizer.RAW_SCHEMA,
        "dataset": summarizer.DATASET,
        "platform_version": summarizer.PLATFORM_VERSION,
        "provider_schema_version": summarizer.PROVIDER_SCHEMA_VERSION,
        "extraction_schema_version": summarizer.EXTRACTION_SCHEMA_VERSION,
        "hbk_sha256": summarizer.HBK_SHA256,
        "provider_sha256": summarizer.PROVIDER_SHA256,
        "backend": name,
        "decision_role": role,
        "baseline_role": "h0",
        "candidate_commit": ("a" if name == "S83-H0" else "b") * 40,
        "candidate_branch": "experiment/base" if name == "S83-H0" else "experiment/control",
        "harness_commit": "c" * 40,
        "harness_file_sha256": {"probe": "e" * 64},
        "manifest_sha256": "d" * 64,
        "host": {"hostname": "test-host"},
        "orchestration_version": summarizer.ORCHESTRATION_VERSION,
        "backend_registry": ["S83-H0", "S83-C0"],
        "availability_context_registry": list(summarizer.AVAILABILITY_CONTEXTS),
        "cache_stance_registry": list(summarizer.CACHE_STANCES),
        "planned_samples_per_row": 2,
        "module_context_filter_used": False,
        "empty_availability_rule": "universal",
        "availability_context": context,
        "cache_stance": stance,
        "sample": sample,
        "iterations": 10,
        "status": "ok",
        "command_template": ["probe", "{context}", "{iterations}"],
        "command": ["probe", context, "10"],
        "machine_state_before": {"load": 0},
        "machine_state_after": {"load": 0},
        "preparation": {"method": stance},
        "transcript": {
            "sha256": digest,
            "baseline_sha256": digest,
            "bytes": 100,
            "item_count": 2,
            "parity_status": "pass",
        },
        "measurement": {
            "schema_version": summarizer.REPORT_SCHEMA,
            "workload_version": summarizer.WORKLOAD_VERSION,
            "backend": name,
            "decision_role": role,
            "baseline_role": "h0",
            "module_context_filter_used": False,
            "empty_availability_rule": "universal",
            "availability_context": context,
            "iterations": 10,
            "counts": {
                "scanned_globals": 4,
                "candidate_methods": 3,
                "returned_objects": 2,
                "universal_objects": 1,
                "explicit_context_objects": 1,
                "excluded_objects": 1,
                "universal_assertion": True,
                "excluded_assertion": True,
            },
            "timings": {
                "entry_to_ready_ns": value,
                "first_enumeration": {
                    "elapsed_ns": value,
                    "ns_per_object": value,
                    "faults": {"minor": value, "major": 0},
                },
                "workload": {
                    "elapsed_ns": value,
                    "average_ns": value,
                    "ns_per_object": value,
                    "faults": {"minor": value, "major": 0},
                },
            },
            "allocations": {
                "enabled": True,
                "first_enumeration": {
                    "allocation_calls": value,
                    "allocated_bytes": value,
                },
                "workload": {
                    "allocation_calls": value,
                    "allocated_bytes": value,
                },
                "final_snapshot": {
                    "current_live_bytes": value,
                    "peak_live_bytes": value,
                },
            },
        },
    }


def complete_records(samples: int = 2) -> list[dict[str, Any]]:
    records = []
    for name, multiplier in (("S83-H0", 1), ("S83-C0", 2)):
        for context in summarizer.AVAILABILITY_CONTEXTS:
            for stance in summarizer.CACHE_STANCES:
                for sample in range(1, samples + 1):
                    records.append(raw_record(name, context, stance, sample, multiplier))
    return records


class BenchmarkContractTests(unittest.TestCase):
    def test_report_validation_strips_transcript_and_preserves_exact_canonical_bytes(self) -> None:
        report = probe_report()
        transcript, stripped = benchmark.validate_report(
            report, backend(), "server", 3, require_allocations=True
        )
        self.assertEqual(
            transcript,
            b'[{"id":"universal"},{"id":"explicit"}]',
        )
        self.assertNotIn("transcript", stripped)

    def test_report_rejects_module_context_kind_metadata(self) -> None:
        report = probe_report()
        report["module_context_kind"] = "server"
        with self.assertRaisesRegex(benchmark.EvidenceError, "ModuleContextKind"):
            benchmark.validate_report(
                report, backend(), "server", 3, require_allocations=True
            )

    def test_report_rejects_missing_real_corpus_guard(self) -> None:
        report = probe_report()
        report["counts"]["excluded_assertion"] = False
        with self.assertRaisesRegex(benchmark.EvidenceError, "guard failed"):
            benchmark.validate_report(
                report, backend(), "server", 3, require_allocations=True
            )

    def test_command_template_substitutes_context_and_iterations(self) -> None:
        self.assertEqual(
            benchmark.command_for(("probe", "{context}", "{iterations}"), "thin_client", 9),
            ["probe", "thin_client", "9"],
        )

    def test_cold_preparation_declares_advisory_eviction(self) -> None:
        with tempfile.NamedTemporaryFile() as source:
            with mock.patch.object(benchmark.subprocess, "run") as run, mock.patch.object(
                benchmark.os, "posix_fadvise"
            ) as advise:
                result = benchmark.prepare_files("cold-best-effort", [Path(source.name)])
        run.assert_called_once_with(("sync",), check=True)
        advise.assert_called_once()
        self.assertEqual(result["claim"], "cold-best-effort")
        self.assertFalse(result["eviction_verified"])


class SummaryContractTests(unittest.TestCase):
    def test_complete_matrix_has_median_mad_and_h0_ratios(self) -> None:
        summary = summarizer.build_summary(complete_records(), expected_samples=2)
        self.assertEqual(len(summary["rows"]), 2 * 9 * 2)
        h0 = summary["rows"][0]
        c0 = next(
            row
            for row in summary["rows"]
            if row["backend"] == "S83-C0"
            and row["availability_context"] == "thin_client"
            and row["cache_stance"] == "warm"
        )
        self.assertEqual(h0["metrics"]["first_enumeration_ns"]["ratio_to_h0"], 1.0)
        self.assertEqual(c0["metrics"]["first_enumeration_ns"]["ratio_to_h0"], 2.0)
        self.assertEqual(c0["metrics"]["first_enumeration_ns"]["mad"], 1.0)

    def test_duplicate_sample_is_rejected(self) -> None:
        records = complete_records()
        records.append(dict(records[0]))
        with self.assertRaisesRegex(summarizer.SummaryError, "duplicate sample"):
            summarizer.build_summary(records, expected_samples=2)

    def test_missing_sample_is_rejected(self) -> None:
        records = complete_records()
        records.pop()
        with self.assertRaisesRegex(summarizer.SummaryError, "expected samples"):
            summarizer.build_summary(records, expected_samples=2)

    def test_candidate_parity_mismatch_is_rejected_even_if_self_declared_pass(self) -> None:
        records = complete_records()
        target = next(
            record
            for record in records
            if record["backend"] == "S83-C0"
            and record["availability_context"] == "thin_client"
            and record["cache_stance"] == "warm"
        )
        target["transcript"]["sha256"] = "e" * 64
        target["transcript"]["baseline_sha256"] = "e" * 64
        with self.assertRaisesRegex(summarizer.SummaryError, "parity drift|differs from S83-H0"):
            summarizer.build_summary(records, expected_samples=2)

    def test_raw_measurement_cannot_retain_transcript(self) -> None:
        records = complete_records()
        records[0]["measurement"]["transcript"] = []
        with self.assertRaisesRegex(summarizer.SummaryError, "retained the large transcript"):
            summarizer.build_summary(records, expected_samples=2)

    def test_summary_has_no_candidate_ordering_fields(self) -> None:
        summary = summarizer.build_summary(complete_records(), expected_samples=2)
        forbidden = {"rank", "ranking", "winner", "priority", "first_place"}

        def keys(value: Any) -> set[str]:
            if isinstance(value, dict):
                return set(value) | set().union(*(keys(nested) for nested in value.values()))
            if isinstance(value, list):
                return set().union(*(keys(nested) for nested in value), set())
            return set()

        self.assertTrue(forbidden.isdisjoint(keys(summary)))


if __name__ == "__main__":
    unittest.main()
