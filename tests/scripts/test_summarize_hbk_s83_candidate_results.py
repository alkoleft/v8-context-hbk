from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "summarize-hbk-s83-candidate-results.py"
HARNESS = "28f29b5a262db362b6b58c8109e6df6c2afbbc44"
DATASET = "shcntx_ru-8.3.27.1859-schema16-extraction11"
SQLITE_SHA = "55c2e09971712a13a49cbcf5889f203d7a9dfcec22aa0d333247ae722f6f0fab"
HBK_SHA = "5bdf0b3ed89932572c012faddc4d05ebfa2986595cf2849b6eb6e5e65a9a4d48"
CONTENT_SHA = "5f66d20509877ac29a83ede2d5178368ed3fd78d7dab0ffbc12df506acc3b1fd"
LOOKUP_SHA = "9b17c7100cd368fe0880e679d66ab8eb7d8505ee617d9fc80b1a9a9d8aa5c5c8"
SEMANTIC_SHA = "1fe7f166caad8e8573b809a97f7104caf85301370f1d34017376bc82ee893a29"
OPERATIONS = [
    ("dictionary_by_id", 20000),
    ("dictionary_by_value", 20000),
    ("dictionary_by_value_miss", 0),
    ("exact_fact_id", 20000),
    ("type_by_name", 20000),
    ("type_template_by_key", 20000),
    ("members_by_owner", 240000),
    ("member_by_owner_name_kind", 20000),
    ("callable_by_owner_name", 20000),
    ("constructors_by_type", 20000),
    ("global_by_domain_name_kind", 20000),
    ("module_context_by_kind", 480000),
    ("query_table_by_name", 20000),
    ("query_field_by_table_name", 20000),
    ("query_param_by_table_name", 20000),
    ("availability_by_fact", 200000),
    ("relation_by_source_kind", 20000),
    ("language_by_name", 0),
    ("enum_by_name", 20000),
    ("query_table_by_syntax", 20000),
    ("query_table_by_identifier", 20000),
    ("exact_fact_id_miss", 0),
    ("type_by_name_miss", 0),
    ("language_by_name_miss", 0),
    ("enum_by_name_miss", 0),
]


def base(backend: str, commit: str, sample: int, stance: str = "warm") -> dict:
    return {
        "schema": "hbk-snapshot-benchmark-raw-v1",
        "backend": backend,
        "cache_stance": stance,
        "dataset": DATASET,
        "platform_version": "8.3.27.1859",
        "provider_schema_version": 16,
        "extraction_schema_version": 11,
        "sqlite_sha256": SQLITE_SHA,
        "hbk_sha256": HBK_SHA,
        "sample": sample,
        "status": "ok",
        "harness_commit": HARNESS,
        "candidate_commit": commit,
        "candidate_branch": f"experiment/{backend}",
        "process": {"maximum_rss_kib": 13000 + sample},
    }


def runtime_record(backend: str, commit: str, sample: int, stance: str) -> dict:
    record = base(backend, commit, sample, stance)
    record["resident_bytes_before"] = {"artifact": 0 if stance == "cold-best-effort" else 100}
    record["resident_bytes_after"] = {"artifact": 100}
    record["measurement"] = {
        "timings": {
            "process_start_to_ready_ns": 20_000_000 + sample,
            "first_lookup": {"elapsed_ns": 3_000 + sample},
            "anchor_resolution": {"elapsed_ns": 10_000 + sample},
            "open": {"faults": {"minor": 200 + sample, "major": 0}},
            "workload": {
                "elapsed_ns": 300_000_000 + sample,
                "operations": [
                    {
                        "name": name,
                        "average_ns": operation_average(name, sample),
                        "observed_total": observed_total,
                    }
                    for name, observed_total in OPERATIONS
                ],
            },
        },
        "smaps": {"after_workload": {"pss_kib": 11000 + sample, "private_kib": 10900 + sample}},
        "footprint": {
            "artifact_bytes": 11_000_000,
            "section_bytes": 10_000_000,
            "dictionary_bytes": 4_000_000,
            "index_bytes": 3_000_000,
        },
        "allocations": {
            "entry_to_ready": {
                "allocation_calls": 10,
                "allocated_bytes": 100,
                "deallocated_bytes": 90,
            },
            "final_snapshot": {"current_live_bytes": 10, "peak_live_bytes": 20},
        },
    }
    return record


def operation_average(name: str, sample: int) -> int:
    if name == "dictionary_by_id":
        return 3
    if name == "dictionary_by_value":
        return 300 + sample
    if name == "dictionary_by_value_miss":
        return 20_000 + sample
    return 100 + sample


def f0_production_record(commit: str, sample: int, backend: str = "s83-f0") -> dict:
    record = base(f"{backend}-produce", commit, sample)
    record["measurement"] = {
        "timings": {
            "total_ns": 700_000_000 + sample,
            "materialize_ns": 600_000_000 + sample,
            "write_publish_ns": 50_000_000 + sample,
            "in_memory_validation_ns": 20_000_000 + sample,
        },
        "footprint": {
            "artifact_bytes": 11_000_000,
            "section_bytes": 10_000_000,
            "dictionary_bytes": 4_000_000,
            "index_bytes": 3_000_000,
        },
        "allocations": {
            "producer": {
                "allocation_calls": 1000,
                "allocated_bytes": 2000,
                "deallocated_bytes": 1000,
            },
            "final_snapshot": {"current_live_bytes": 100, "peak_live_bytes": 200},
        },
    }
    return record


def a0_production_record(commit: str, sample: int, backend: str = "s83-a0") -> dict:
    record = base(f"{backend}-produce", commit, sample)
    record["measurement"] = {
        "schema": "hbk-s83-a0-produce/v1",
        "total_ns": 710_000_000 + sample,
        "materialize_ns": 610_000_000 + sample,
        "serialize_ns": 25_000_000 + sample,
        "validate_ns": 15_000_000 + sample,
        "write_ns": 55_000_000 + sample,
        "artifact_bytes": 13_000_000,
        "archive_footprint": {
            "artifact_bytes": 13_000_000,
            "dictionary_text_bytes": 4_100_000,
            "sorted_index_estimated_fixed_bytes": 2_300_000,
        },
        "allocation_phases": {
            "total": {
                "allocation_calls": 1001,
                "allocated_bytes": 2001,
                "deallocated_bytes": 1001,
            },
            "final_snapshot": {"current_live_bytes": 101, "peak_live_bytes": 201},
        },
    }
    return record


def allocation_record(backend: str, commit: str, sample: int) -> dict:
    record = runtime_record(backend, commit, sample, "warm")
    record["scenario"] = "allocation-profile"
    return record


def four_reader_record(backend: str, commit: str, sample: int) -> dict:
    record = base(backend, commit, sample)
    record["scenario"] = "aggregate-four-reader-pss"
    record["aggregate"] = {
        "rss_kib": 40000 + sample,
        "pss_kib": 50000 + sample,
        "private_kib": 49000 + sample,
        "shared_kib": 1000,
        "anonymous_kib": 100,
    }
    return record


def complete_candidate(backend: str, commit: str) -> list[dict]:
    records = []
    records.extend(runtime_record(backend, commit, i, "warm") for i in range(1, 10))
    records.extend(runtime_record(backend, commit, i, "cold-best-effort") for i in range(1, 10))
    producer = a0_production_record if backend == "s83-a0" else f0_production_record
    records.extend(producer(commit, i, backend) for i in range(1, 10))
    records.extend(allocation_record(backend, commit, i) for i in range(1, 4))
    records.extend(
        producer(commit, i, backend) | {"scenario": "allocation-profile"}
        for i in range(1, 4)
    )
    records.extend(four_reader_record(backend, commit, i) for i in range(1, 4))
    return records


def write_jsonl(path: Path, records: list[dict]) -> None:
    with path.open("w", encoding="utf-8") as stream:
        for record in records:
            stream.write(json.dumps(record, ensure_ascii=False, separators=(",", ":")) + "\n")


def metric(median: int, mad: int = 0, noisy: bool = False) -> dict:
    return {
        "samples": 9,
        "median": median,
        "mad": mad,
        "mad_ratio": 0 if median == 0 else mad / median,
        "noisy": noisy,
    }


def baseline_group(backend: str, stance: str) -> dict:
    operations = {
        name: metric(operation_average(name, 0), 2)
        | {"observed_totals": [observed_total]}
        for name, observed_total in OPERATIONS
    }
    operations["dictionary_by_id"] = metric(0) | {"observed_totals": [20000]}
    operations["dictionary_by_value"] = metric(650, 10) | {"observed_totals": [20000]}
    operations["dictionary_by_value_miss"] = metric(45_000, 100) | {"observed_totals": [0]}
    return {
        "identity": {
            "dataset": DATASET,
            "backend": backend,
            "cache_stance": stance,
            "candidate_branch": "experiment/hbk-zero-copy-base",
            "candidate_commit": HARNESS,
        },
        "samples": 9,
        "metrics": {
            "ready_ns": metric(40_000_000 if backend == "cache-owned" else 580_000_000, 1000),
            "materialize_ns": metric(38_000_000),
            "first_lookup_ns": metric(20_000, 2_000, noisy=True),
            "anchor_resolution_ns": metric(12_000),
            "workload_ns": metric(320_000_000),
            "peak_rss_kib": metric(30_000),
            "workload_pss_kib": metric(20_000),
            "workload_private_kib": metric(19_000),
            "open_minor_faults": metric(200),
            "open_major_faults": metric(0),
            "file_resident_growth_bytes": metric(100),
        },
        "operations": operations,
    }


def production_baseline_group() -> dict:
    group = baseline_group("cache-owned-produce", "warm")
    group["operations"] = {}
    group["metrics"] = {
        "ready_ns": metric(650_000_000),
        "materialize_ns": metric(590_000_000),
        "artifact_write_ns": metric(60_000_000),
        "peak_rss_kib": metric(80_000),
        "artifact_bytes": metric(11_000_000),
    }
    return group


def allocation_baseline_group(backend: str) -> dict:
    group = baseline_group(backend, "warm")
    group["operations"] = {}
    group["metrics"] = {
        "entry_allocation_calls": metric(2000),
        "entry_allocated_bytes": metric(4000),
        "entry_deallocated_bytes": metric(3000),
        "final_live_bytes": metric(100),
        "peak_live_bytes": metric(200),
    }
    return group


def aggregate_baseline_group(backend: str) -> dict:
    group = baseline_group(backend, "warm")
    group["operations"] = {}
    group["metrics"] = {
        "rss_kib": metric(60_000),
        "pss_kib": metric(50_000),
        "private_kib": metric(49_000),
    }
    return group


def write_baseline_summary(path: Path) -> None:
    groups = [
        baseline_group("cache-owned", "warm"),
        baseline_group("cache-owned", "cold-best-effort"),
        production_baseline_group(),
        baseline_group("sql-owned", "warm"),
        baseline_group("sql-owned", "cold-best-effort"),
    ]
    allocation = [
        allocation_baseline_group("cache-owned"),
        allocation_baseline_group("cache-owned-produce"),
        allocation_baseline_group("sql-owned"),
    ]
    aggregate = [
        aggregate_baseline_group("cache-owned"),
        aggregate_baseline_group("sql-owned"),
    ]

    def keyed(items: list[dict]) -> dict:
        return {
            json.dumps(
                [
                    item["identity"]["dataset"],
                    item["identity"]["backend"],
                    item["identity"]["cache_stance"],
                    item["identity"]["candidate_branch"],
                    item["identity"]["candidate_commit"],
                ],
                separators=(",", ":"),
            ): item
            for item in items
        }

    path.write_text(
        json.dumps(
            {
                "schema": "hbk-snapshot-benchmark-summary-v1",
                "harness_commit": HARNESS,
                "ranked": False,
                "groups": keyed(groups),
                "allocation_profiles": keyed(allocation),
                "aggregate_four_reader": keyed(aggregate),
                "parity": [],
            },
            separators=(",", ":"),
        ),
        encoding="utf-8",
    )


def storage_parity(backend: str, commit: str) -> dict:
    return {
        "backend": backend,
        "status": "pass",
        "candidate_commit": commit,
        "exit_statuses": [0, 0, 0, 0, 0],
        "content_sha256": CONTENT_SHA,
        "lookup_sha256": LOOKUP_SHA,
        "content_bytes": 57486556,
        "lookup_bytes": 88520585,
        "content_records": 176793,
        "lookup_records": 276415,
    }


def semantic_output() -> dict:
    return {
        "sha256": SEMANTIC_SHA,
        "size": 769824709,
        "records": 742872,
        "baseline_byte_equal": True,
        "sequential_byte_equal": True,
    }


class S83CandidateSummaryTests(unittest.TestCase):
    def run_script(self, directory: Path, raw: Path, *extra: str) -> subprocess.CompletedProcess[str]:
        baseline = directory / "baseline-summary.json"
        if not baseline.exists():
            write_baseline_summary(baseline)
        return subprocess.run(
            [
                sys.executable,
                str(SCRIPT),
                str(raw),
                "--baseline-summary",
                str(baseline),
                "--json",
                str(directory / "summary.json"),
                "--markdown",
                str(directory / "summary.md"),
                *extra,
            ],
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )

    def test_summarizes_complete_f0_and_a0_native_records(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            raw = directory / "raw.jsonl"
            storage = directory / "storage.jsonl"
            semantic = directory / "semantic.jsonl"
            write_jsonl(
                raw,
                complete_candidate("s83-f0", "f0" * 20)
                + complete_candidate("s83-a0", "a0" * 20),
            )
            write_jsonl(
                storage,
                [
                    storage_parity("s83-f0", "f0" * 20),
                    storage_parity("s83-a0", "a0" * 20),
                ],
            )
            write_jsonl(
                semantic,
                [
                    {
                        "backend": "s83-f0-semantic",
                        "status": "pass",
                        "candidate_commit": "f0" * 20,
                        "expected_transcript_sha256": SEMANTIC_SHA,
                        "expected_transcript_records": 742872,
                        "expected_transcript_size": 769824709,
                        "exit_statuses": [0, 0, 0, 0, 0],
                        "outputs": [semantic_output() for _ in range(5)],
                    },
                    {
                        "backend": "s83-a0-semantic",
                        "status": "pass",
                        "candidate_commit": "a0" * 20,
                        "expected_transcript_sha256": SEMANTIC_SHA,
                        "expected_transcript_records": 742872,
                        "expected_transcript_size": 769824709,
                        "exit_statuses": [0, 0, 0, 0, 0],
                        "outputs": [semantic_output() for _ in range(5)],
                    },
                ],
            )
            completed = self.run_script(
                directory,
                raw,
                "--storage-parity-raw",
                str(storage),
                "--semantic-parity-raw",
                str(semantic),
            )
            self.assertEqual(completed.returncode, 0, completed.stderr)
            summary = json.loads((directory / "summary.json").read_text(encoding="utf-8"))
            self.assertTrue(summary["unranked"])
            self.assertFalse(summary["ranked"])
            self.assertEqual(summary["selection"], "pending-user-decision")
            self.assertEqual([c["backend"] for c in summary["candidates"]], ["s83-f0", "s83-a0"])
            a0 = summary["candidates"][1]
            self.assertIn("serialize_ns", a0["production"]["metrics"])
            self.assertEqual(a0["storage_parity"]["status"], "pass")
            self.assertEqual(a0["semantic_parity"]["status"], "pass")
            self.assertIn("operation_ceiling", a0)
            self.assertEqual(a0["gates"]["warm_reverse_dictionary_hit_ns"]["threshold"], 458)
            self.assertEqual(
                a0["gates"]["cold-best-effort_reverse_dictionary_hit_ns"]["threshold"],
                458,
            )
            self.assertEqual(
                a0["gates"]["cold-best-effort_reverse_dictionary_miss_ns"]["threshold"],
                24048,
            )
            self.assertFalse(a0["registry_order_is_rank"])

    def test_rejects_incomplete_official_counts(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            raw = directory / "raw.jsonl"
            records = complete_candidate("s83-f0", "f0" * 20)
            records = [record for record in records if not (record.get("cache_stance") == "cold-best-effort" and record.get("sample") == 9)]
            write_jsonl(raw, records)
            completed = self.run_script(directory, raw)
            self.assertNotEqual(completed.returncode, 0)
            self.assertIn("expected 9", completed.stderr)

    def test_rejects_failed_official_record(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            raw = directory / "raw.jsonl"
            records = complete_candidate("s83-f0", "f0" * 20)
            records[0]["status"] = "failed"
            write_jsonl(raw, records)
            completed = self.run_script(directory, raw)
            self.assertNotEqual(completed.returncode, 0)
            self.assertIn("failed official record", completed.stderr)

    def test_rejects_wrong_operation_totals_and_set(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            raw = directory / "raw.jsonl"
            records = complete_candidate("s83-f0", "f0" * 20)
            first_runtime = next(
                record
                for record in records
                if record.get("backend") == "s83-f0"
                and record.get("cache_stance") == "warm"
                and record.get("scenario") is None
            )
            first_runtime["measurement"]["timings"]["workload"]["operations"][0]["observed_total"] = 123
            write_jsonl(raw, records)
            completed = self.run_script(directory, raw)
            self.assertNotEqual(completed.returncode, 0)
            self.assertIn("observed_total", completed.stderr)

            first_runtime["measurement"]["timings"]["workload"]["operations"][0]["observed_total"] = 20000
            for record in records:
                if (
                    record.get("backend") == "s83-f0"
                    and record.get("cache_stance") == "warm"
                    and record.get("scenario") is None
                ):
                    record["measurement"]["timings"]["workload"]["operations"].pop()
            write_jsonl(raw, records)
            completed = self.run_script(directory, raw)
            self.assertNotEqual(completed.returncode, 0)
            self.assertIn("operation set mismatch", completed.stderr)

    def test_per_operation_formula_reports_failures(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            raw = directory / "raw.jsonl"
            records = complete_candidate("s83-f0", "f0" * 20)
            for record in records:
                if (
                    record.get("backend") == "s83-f0"
                    and record.get("cache_stance") == "warm"
                    and record.get("scenario") is None
                ):
                    for operation in record["measurement"]["timings"]["workload"]["operations"]:
                        if operation["name"] == "exact_fact_id":
                            operation["average_ns"] = 1000
            write_jsonl(raw, records)
            completed = self.run_script(directory, raw)
            self.assertEqual(completed.returncode, 0, completed.stderr)
            summary = json.loads((directory / "summary.json").read_text(encoding="utf-8"))
            candidate = summary["candidates"][0]
            self.assertEqual(candidate["operation_ceiling"]["warm"]["status"], "fail")
            self.assertIn("exact_fact_id", candidate["operation_ceiling"]["warm"]["failed_operations"])

    def test_noisy_absolute_first_lookup_keeps_pass_fail_status(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            raw = directory / "raw.jsonl"
            records = complete_candidate("s83-f0", "f0" * 20)
            noisy_values = [1000, 20_000, 1000, 20_000, 10_000, 1000, 20_000, 1000, 20_000]
            for record, value in zip(
                (
                    r
                    for r in records
                    if r.get("backend") == "s83-f0"
                    and r.get("cache_stance") == "warm"
                    and r.get("scenario") is None
                ),
                noisy_values,
            ):
                record["measurement"]["timings"]["first_lookup"]["elapsed_ns"] = value
            write_jsonl(raw, records)
            completed = self.run_script(directory, raw)
            self.assertEqual(completed.returncode, 0, completed.stderr)
            summary = json.loads((directory / "summary.json").read_text(encoding="utf-8"))
            gate = summary["candidates"][0]["gates"]["warm_first_lookup_ns"]
            self.assertEqual(gate["status"], "pass")
            self.assertTrue(gate["noisy"])

    def test_derived_backend_registry_mapping(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            raw = directory / "raw.jsonl"
            records = complete_candidate("s83-l1", "l1" * 20)
            write_jsonl(raw, records)
            completed = self.run_script(directory, raw)
            self.assertEqual(completed.returncode, 0, completed.stderr)
            summary = json.loads((directory / "summary.json").read_text(encoding="utf-8"))
            self.assertEqual(summary["candidates"][0]["backend"], "s83-l1")
            self.assertEqual(summary["candidates"][0]["registry_presentation_order"], 2)

    def test_parity_backend_maps_all_registry_derived_labels(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            raw = directory / "raw.jsonl"
            storage = directory / "storage.jsonl"
            records = complete_candidate("s83-r1", "r1" * 20)
            write_jsonl(raw, records)
            write_jsonl(storage, [storage_parity("s83-r1-artifact", "r1" * 20)])
            completed = self.run_script(directory, raw, "--storage-parity-raw", str(storage))
            self.assertEqual(completed.returncode, 0, completed.stderr)
            summary = json.loads((directory / "summary.json").read_text(encoding="utf-8"))
            self.assertEqual(summary["candidates"][0]["storage_parity"]["status"], "pass")

    def test_rejects_missing_required_metric_value(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            raw = directory / "raw.jsonl"
            records = complete_candidate("s83-f0", "f0" * 20)
            first_runtime = next(
                record
                for record in records
                if record.get("backend") == "s83-f0"
                and record.get("cache_stance") == "warm"
                and record.get("scenario") is None
            )
            del first_runtime["measurement"]["timings"]["first_lookup"]["elapsed_ns"]
            write_jsonl(raw, records)
            completed = self.run_script(directory, raw)
            self.assertNotEqual(completed.returncode, 0)
            self.assertIn("runtime.first_lookup_ns", completed.stderr)

    def test_rejects_missing_operation_observed_total(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            raw = directory / "raw.jsonl"
            records = complete_candidate("s83-f0", "f0" * 20)
            first_runtime = next(
                record
                for record in records
                if record.get("backend") == "s83-f0"
                and record.get("cache_stance") == "warm"
                and record.get("scenario") is None
            )
            del first_runtime["measurement"]["timings"]["workload"]["operations"][0]["observed_total"]
            write_jsonl(raw, records)
            completed = self.run_script(directory, raw)
            self.assertNotEqual(completed.returncode, 0)
            self.assertIn("observed_total", completed.stderr)

    def test_rejects_missing_corpus_identity_and_unknown_backend(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            raw = directory / "raw.jsonl"
            records = complete_candidate("s83-f0", "f0" * 20)
            del records[0]["sqlite_sha256"]
            write_jsonl(raw, records)
            completed = self.run_script(directory, raw)
            self.assertNotEqual(completed.returncode, 0)
            self.assertIn("incomplete S83 corpus identity", completed.stderr)

            records = complete_candidate("s83-x9", "x9" * 20)
            write_jsonl(raw, records)
            completed = self.run_script(directory, raw)
            self.assertNotEqual(completed.returncode, 0)
            self.assertIn("unknown S83 resource backend", completed.stderr)

    def test_rejects_unproven_explicit_pass_and_bad_parity_raw(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            raw = directory / "raw.jsonl"
            storage = directory / "storage.jsonl"
            records = complete_candidate("s83-f0", "f0" * 20)
            write_jsonl(raw, records)
            completed = self.run_script(directory, raw, "--storage-parity-status", "s83-f0=pass")
            self.assertNotEqual(completed.returncode, 0)
            self.assertIn("cannot claim pass", completed.stderr)

            bad = storage_parity("s83-f0", "f0" * 20)
            bad["exit_statuses"] = [0, 0, 0, 0, 1]
            write_jsonl(storage, [bad])
            completed = self.run_script(directory, raw, "--storage-parity-raw", str(storage))
            self.assertEqual(completed.returncode, 0, completed.stderr)
            summary = json.loads((directory / "summary.json").read_text(encoding="utf-8"))
            self.assertEqual(summary["candidates"][0]["storage_parity"]["status"], "fail")

    def test_rejects_bad_semantic_raw_proof(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            raw = directory / "raw.jsonl"
            semantic = directory / "semantic.jsonl"
            records = complete_candidate("s83-f0", "f0" * 20)
            write_jsonl(raw, records)
            output = semantic_output()
            output["sequential_byte_equal"] = False
            write_jsonl(
                semantic,
                [
                    {
                        "backend": "s83-f0-semantic",
                        "status": "pass",
                        "candidate_commit": "f0" * 20,
                        "expected_transcript_sha256": SEMANTIC_SHA,
                        "expected_transcript_records": 742872,
                        "expected_transcript_size": 769824709,
                        "exit_statuses": [0, 0, 0, 0, 0],
                        "outputs": [output, *[semantic_output() for _ in range(4)]],
                    }
                ],
            )
            completed = self.run_script(directory, raw, "--semantic-parity-raw", str(semantic))
            self.assertEqual(completed.returncode, 0, completed.stderr)
            summary = json.loads((directory / "summary.json").read_text(encoding="utf-8"))
            self.assertEqual(summary["candidates"][0]["semantic_parity"]["status"], "fail")


if __name__ == "__main__":
    unittest.main()
