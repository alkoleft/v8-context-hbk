#!/usr/bin/env python3
"""Candidate-only orchestration around the frozen S83 benchmark harness.

The frozen shell harness already accepts arbitrary runtime/producer commands.
Its allocation and four-reader helpers are intentionally H0/C0-specific, so
this script supplies those two outer scenarios plus concurrent parity without
changing or post-processing the measured values.
"""

from __future__ import annotations

import argparse
import filecmp
import hashlib
import json
import os
import platform
import re
import shutil
import subprocess
import sys
import time
from pathlib import Path
from typing import Any, Sequence


HARNESS_COMMIT = "28f29b5a262db362b6b58c8109e6df6c2afbbc44"
DATASET = "shcntx_ru-8.3.27.1859-schema16-extraction11"
PLATFORM_VERSION = "8.3.27.1859"
PROVIDER_SCHEMA_VERSION = 16
EXTRACTION_SCHEMA_VERSION = 11
HBK_PATH = Path("/opt/1cv8/x86_64/8.3.27.1859/shcntx_ru.hbk")
HBK_SHA256 = "5bdf0b3ed89932572c012faddc4d05ebfa2986595cf2849b6eb6e5e65a9a4d48"
PROVIDER_RELATIVE_PATH = Path(
    "target/snapshot-materialization/"
    "shcntx_ru.8.3.27.1859.schema16.release.sqlite"
)
PROVIDER_SHA256 = (
    "55c2e09971712a13a49cbcf5889f203d7a9dfcec22aa0d333247ae722f6f0fab"
)
BASELINE_CONTENT_SHA256 = (
    "5f66d20509877ac29a83ede2d5178368ed3fd78d7dab0ffbc12df506acc3b1fd"
)
BASELINE_LOOKUP_SHA256 = (
    "9b17c7100cd368fe0880e679d66ab8eb7d8505ee617d9fc80b1a9a9d8aa5c5c8"
)
BASELINE_CONTENT_BYTES = 57_486_556
BASELINE_LOOKUP_BYTES = 88_520_585
BASELINE_CONTENT_RECORDS = 176_793
BASELINE_LOOKUP_RECORDS = 276_415
RAW_SCHEMA = "hbk-snapshot-benchmark-raw-v1"
BACKEND_PATTERN = re.compile(r"^[A-Za-z0-9._-]+$")
FROZEN_HARNESS_PATHS = (
    "scripts/benchmark-hbk-snapshot-candidates.sh",
    "scripts/summarize-hbk-snapshot-results.py",
    "crates/syntax-helper-search/examples/measure_hbk_snapshot_scenario.rs",
    "crates/syntax-helper-search/examples/dump_hbk_snapshot_oracle.rs",
    "crates/syntax-helper-search/src/snapshot/experiment_allocator.rs",
    "crates/syntax-helper-search/src/snapshot/experiment_oracle.rs",
)


def run_text(command: Sequence[str], cwd: Path | None = None) -> str:
    return subprocess.check_output(command, cwd=cwd, text=True).strip()


def git_text(worktree: Path, *args: str) -> str:
    return run_text(("git", "-C", str(worktree), *args))


def verify_candidate_worktree(worktree: Path) -> tuple[str, str]:
    if git_text(worktree, "status", "--porcelain"):
        raise RuntimeError(f"candidate worktree is dirty: {worktree}")
    return (
        git_text(worktree, "rev-parse", "HEAD"),
        git_text(worktree, "branch", "--show-current"),
    )


def verify_harness_commit(repo: Path) -> None:
    actual = git_text(
        repo,
        "log",
        "-1",
        "--format=%H",
        "--",
        *FROZEN_HARNESS_PATHS,
    )
    if actual != HARNESS_COMMIT:
        raise RuntimeError(
            f"frozen harness mismatch: expected {HARNESS_COMMIT}, got {actual}"
        )
    dirty = git_text(repo, "status", "--porcelain", "--", *FROZEN_HARNESS_PATHS)
    if dirty:
        raise RuntimeError(f"frozen harness files have uncommitted changes:\n{dirty}")


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def verify_frozen_inputs(repo: Path) -> Path:
    provider = repo / PROVIDER_RELATIVE_PATH
    for path, expected in ((HBK_PATH, HBK_SHA256), (provider, PROVIDER_SHA256)):
        if not path.is_file():
            raise RuntimeError(f"missing frozen input: {path}")
        actual = sha256(path)
        if actual != expected:
            raise RuntimeError(
                f"frozen input checksum mismatch for {path}: "
                f"expected {expected}, got {actual}"
            )
    return provider


def verify_no_candidate_provider(candidate_worktree: Path) -> None:
    candidate_provider = candidate_worktree / PROVIDER_RELATIVE_PATH
    if candidate_provider.exists():
        raise RuntimeError(
            "candidate worktree contains a fallback SQLite provider path: "
            f"{candidate_provider}"
        )


def verify_baseline_file(
    path: Path, expected_sha: str, expected_bytes: int, expected_records: int
) -> None:
    if not path.is_file():
        raise RuntimeError(f"frozen S83 parity file is missing: {path}")
    actual_bytes = path.stat().st_size
    if actual_bytes != expected_bytes:
        raise RuntimeError(
            f"frozen S83 parity byte-size mismatch for {path}: "
            f"expected {expected_bytes}, got {actual_bytes}"
        )
    actual_records = count_lines(path)
    if actual_records != expected_records:
        raise RuntimeError(
            f"frozen S83 parity record-count mismatch for {path}: "
            f"expected {expected_records}, got {actual_records}"
        )
    actual_sha = sha256(path)
    if actual_sha != expected_sha:
        raise RuntimeError(
            f"frozen S83 parity checksum mismatch for {path}: "
            f"expected {expected_sha}, got {actual_sha}"
        )


def host_environment() -> dict[str, str]:
    return {
        "kernel": f"{platform.system()} {platform.release()}",
        "architecture": platform.machine(),
        "rustc": run_text(("rustc", "--version")),
        "cargo": run_text(("cargo", "--version")),
    }


def machine_state() -> dict[str, Any]:
    load_one, load_five, load_fifteen, scheduler_tasks, last_pid = (
        Path("/proc/loadavg").read_text(encoding="ascii").split()
    )
    runnable, total = scheduler_tasks.split("/", 1)
    uptime, idle = Path("/proc/uptime").read_text(encoding="ascii").split()[:2]
    memory: dict[str, int] = {}
    wanted = {
        "MemAvailable": "available_kib",
        "MemFree": "free_kib",
        "Buffers": "buffers_kib",
        "Cached": "cached_kib",
        "SwapFree": "swap_free_kib",
        "Dirty": "dirty_kib",
    }
    for line in Path("/proc/meminfo").read_text(encoding="ascii").splitlines():
        name, value, *_unit = line.replace(":", "").split()
        if name in wanted:
            memory[wanted[name]] = int(value)
    return {
        "captured_unix_ns": str(time.time_ns()),
        "logical_cpus": os.cpu_count(),
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
        "uptime": {
            "uptime_seconds": float(uptime),
            "idle_seconds": float(idle),
        },
        "memory": memory,
    }


def warm_file(path: Path) -> None:
    if not path.is_file():
        raise RuntimeError(f"cannot warm missing file: {path}")
    with path.open("rb", buffering=0) as stream:
        while stream.read(8 * 1024 * 1024):
            pass


def smaps(pid: int) -> dict[str, int]:
    values = {
        "rss_kib": 0,
        "pss_kib": 0,
        "private_kib": 0,
        "shared_kib": 0,
        "anonymous_kib": 0,
    }
    for line in Path(f"/proc/{pid}/smaps_rollup").read_text().splitlines():
        parts = line.split()
        if len(parts) < 2:
            continue
        key = parts[0]
        if key not in {
            "Rss:",
            "Pss:",
            "Private_Clean:",
            "Private_Dirty:",
            "Shared_Clean:",
            "Shared_Dirty:",
            "Anonymous:",
        }:
            continue
        value = int(parts[1])
        if key == "Rss:":
            values["rss_kib"] = value
        elif key == "Pss:":
            values["pss_kib"] = value
        elif key in ("Private_Clean:", "Private_Dirty:"):
            values["private_kib"] += value
        elif key in ("Shared_Clean:", "Shared_Dirty:"):
            values["shared_kib"] += value
        elif key == "Anonymous:":
            values["anonymous_kib"] = value
    return {"pid": pid, **values}


class Evidence:
    def __init__(self, args: argparse.Namespace) -> None:
        self.repo = args.repo.resolve()
        self.results_root = args.results_root.resolve()
        self.candidate_worktree = args.candidate_worktree.resolve()
        verify_harness_commit(self.repo)
        self.candidate_commit, self.candidate_branch = verify_candidate_worktree(
            self.candidate_worktree
        )
        self.provider = verify_frozen_inputs(self.repo)
        if not BACKEND_PATTERN.fullmatch(args.backend):
            raise RuntimeError(f"unsafe backend label: {args.backend!r}")
        self.backend = args.backend
        self.raw_results = (
            args.raw_results.resolve()
            if args.raw_results
            else self.results_root
            / "results"
            / f"raw-{self.backend}-{self.candidate_commit[:7]}.jsonl"
        )
        self.host = host_environment()

    def base(self, scenario: str, sample: int | None = None) -> dict[str, Any]:
        record: dict[str, Any] = {
            "schema": RAW_SCHEMA,
            "backend": self.backend,
            "dataset": DATASET,
            "platform_version": PLATFORM_VERSION,
            "provider_schema_version": PROVIDER_SCHEMA_VERSION,
            "extraction_schema_version": EXTRACTION_SCHEMA_VERSION,
            "sqlite_sha256": PROVIDER_SHA256,
            "hbk_sha256": HBK_SHA256,
            "build_profile": "release",
            "host": self.host,
            "scenario": scenario,
            "harness_commit": HARNESS_COMMIT,
            "candidate_commit": self.candidate_commit,
            "candidate_branch": self.candidate_branch,
            "orchestration": "candidate-outer-driver-v1",
        }
        if sample is not None:
            record["sample"] = sample
        return record

    def append(self, record: dict[str, Any]) -> None:
        self.raw_results.parent.mkdir(parents=True, exist_ok=True)
        with self.raw_results.open("a", encoding="utf-8") as stream:
            json.dump(record, stream, ensure_ascii=False, separators=(",", ":"))
            stream.write("\n")
            stream.flush()
            os.fsync(stream.fileno())
        print(json.dumps(record, ensure_ascii=False, separators=(",", ":")))


def substitute_sample(command: Sequence[str], sample: int) -> list[str]:
    return [part.replace("{sample}", str(sample)) for part in command]


def allocation_instrumentation_enabled(measurement: dict[str, Any]) -> bool:
    for field in ("allocations", "allocation_phases"):
        evidence = measurement.get(field)
        if isinstance(evidence, dict) and evidence.get("enabled") is True:
            return True
    return False


def allocation(args: argparse.Namespace, evidence: Evidence) -> None:
    if not args.command:
        raise RuntimeError("allocation requires a command after --")
    run_dir = evidence.results_root / "runs"
    log_dir = evidence.results_root / "logs"
    run_dir.mkdir(parents=True, exist_ok=True)
    log_dir.mkdir(parents=True, exist_ok=True)
    for sample in range(1, args.runs + 1):
        command = substitute_sample(args.command, sample)
        warm_paths = [
            Path(value.replace("{sample}", str(sample))) for value in args.warm_path
        ]
        record = evidence.base("allocation-profile", sample)
        record.update(
            {
                "cache_stance": "warm",
                "instrumentation": "counting-system-global-allocator",
                "command": command,
                "machine_state_before": machine_state(),
            }
        )
        try:
            for path in warm_paths:
                warm_file(path)
            record["machine_state_before"] = machine_state()
            completed = subprocess.run(command, text=True, capture_output=True, check=False)
        except (OSError, RuntimeError) as error:
            record.update(
                {
                    "status": "failed",
                    "error": f"allocation orchestration failed: {error}",
                    "machine_state_after": machine_state(),
                }
            )
            evidence.append(record)
            raise RuntimeError(
                f"allocation orchestration failed for sample {sample}: {error}"
            ) from error
        after = machine_state()
        (run_dir / f"allocation.{evidence.backend}.{sample}.json").write_text(
            completed.stdout, encoding="utf-8"
        )
        (log_dir / f"allocation.{evidence.backend}.{sample}.stderr.log").write_text(
            completed.stderr, encoding="utf-8"
        )
        record["machine_state_after"] = after
        if completed.returncode:
            record.update({"status": "failed", "exit_status": completed.returncode})
            evidence.append(record)
            raise RuntimeError(f"allocation command failed for sample {sample}")
        try:
            measurement = json.loads(completed.stdout)
        except json.JSONDecodeError as error:
            record.update({"status": "failed", "error": f"invalid measurement JSON: {error}"})
            evidence.append(record)
            raise
        if not allocation_instrumentation_enabled(measurement):
            record.update(
                {
                    "status": "failed",
                    "error": "candidate allocation instrumentation is not enabled",
                    "measurement": measurement,
                }
            )
            evidence.append(record)
            raise RuntimeError("candidate allocation instrumentation is not enabled")
        record.update({"status": "ok", "measurement": measurement})
        evidence.append(record)


def wait_for_ready(ready_files: Sequence[Path], processes: Sequence[subprocess.Popen[str]]) -> None:
    deadline = time.monotonic() + 30.0
    while time.monotonic() < deadline:
        if all(path.is_file() for path in ready_files):
            return
        failed = [process.returncode for process in processes if process.poll() is not None]
        if failed:
            raise RuntimeError(f"reader exited before hold point: {failed}")
        time.sleep(0.1)
    raise RuntimeError("four readers did not reach the hold point")


def multi_reader(args: argparse.Namespace, evidence: Evidence) -> None:
    binary = args.candidate_bin.resolve()
    artifact = args.artifact.resolve()
    if not os.access(binary, os.X_OK):
        raise RuntimeError(f"candidate binary is not executable: {binary}")
    warm_file(artifact)
    for sample in range(1, args.runs + 1):
        directory = (
            evidence.results_root / "runs" / f"multi-reader.{evidence.backend}.{sample}"
        )
        directory.mkdir(parents=True, exist_ok=True)
        ready_files = [directory / f"ready.{index}" for index in range(1, 5)]
        for path in ready_files:
            path.unlink(missing_ok=True)
        processes: list[subprocess.Popen[str]] = []
        stdout_streams = []
        stderr_streams = []
        before = machine_state()
        orchestration_error: BaseException | None = None
        at_hold: dict[str, Any] | None = None
        per_process: list[dict[str, int]] = []
        statuses: list[int] = []
        try:
            for index, ready_file in enumerate(ready_files, start=1):
                stdout_stream = (directory / f"stdout.{index}.json").open("w")
                stderr_stream = (directory / f"stderr.{index}.log").open("w")
                stdout_streams.append(stdout_stream)
                stderr_streams.append(stderr_stream)
                env = os.environ.copy()
                env["HBK_BENCH_HOLD_MS"] = str(args.hold_ms)
                env["HBK_BENCH_READY_FILE"] = str(ready_file)
                processes.append(
                    subprocess.Popen(
                        (str(binary), str(artifact), str(args.iterations)),
                        stdout=stdout_stream,
                        stderr=stderr_stream,
                        text=True,
                        env=env,
                    )
                )
            wait_for_ready(ready_files, processes)
            at_hold = machine_state()
            per_process = [smaps(process.pid) for process in processes]
            statuses = [process.wait() for process in processes]
        except BaseException as error:
            orchestration_error = error
            for process in processes:
                if process.poll() is None:
                    process.terminate()
            for process in processes:
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    process.kill()
            statuses = [
                process.returncode if process.returncode is not None else -1
                for process in processes
            ]
        finally:
            after = machine_state()
            for stream in (*stdout_streams, *stderr_streams):
                stream.close()
        outputs_valid = True
        output_error: str | None = None
        if orchestration_error is None:
            for index in range(1, 5):
                try:
                    json.loads((directory / f"stdout.{index}.json").read_text())
                except (OSError, json.JSONDecodeError) as error:
                    outputs_valid = False
                    output_error = str(error)
                    break
        record = evidence.base("aggregate-four-reader-pss", sample)
        record.update(
            {
                "cache_stance": "warm",
                "command": [str(binary), str(artifact), str(args.iterations)],
                "machine_state_before": before,
                "machine_state_at_hold": at_hold,
                "machine_state_after": after,
                "exit_statuses": statuses,
                "per_process": per_process,
                "aggregate": {
                    key: sum(row[key] for row in per_process)
                    for key in (
                        "rss_kib",
                        "pss_kib",
                        "private_kib",
                        "shared_kib",
                        "anonymous_kib",
                    )
                },
                "status": (
                    "ok"
                    if orchestration_error is None
                    and outputs_valid
                    and len(statuses) == 4
                    and all(status == 0 for status in statuses)
                    else "failed"
                ),
            }
        )
        if orchestration_error is not None:
            record["error"] = str(orchestration_error)
        elif not outputs_valid:
            record["error"] = f"invalid reader JSON: {output_error}"
        evidence.append(record)
        if record["status"] != "ok":
            raise RuntimeError(f"four-reader command failed for sample {sample}")


def isolated_oracle_command(
    oracle: Path, artifact: Path, content: Path, lookups: Path, provider: Path
) -> list[str]:
    bwrap = shutil.which("bwrap")
    if bwrap is None:
        raise RuntimeError("bwrap is required for the no-fallback parity probe")
    return [
        bwrap,
        "--die-with-parent",
        "--ro-bind",
        "/",
        "/",
        "--dev-bind",
        "/dev",
        "/dev",
        "--proc",
        "/proc",
        "--chdir",
        "/",
        "--bind",
        str(content.parent),
        str(content.parent),
        "--bind",
        str(artifact.parent),
        str(artifact.parent),
        "--ro-bind",
        str(artifact),
        str(artifact),
        "--bind",
        "/dev/null",
        str(HBK_PATH),
        "--bind",
        "/dev/null",
        str(provider),
        "--",
        str(oracle),
        str(artifact),
        str(content),
        str(lookups),
    ]


def count_lines(path: Path) -> int:
    with path.open("rb") as stream:
        return sum(1 for _ in stream)


def parity(args: argparse.Namespace, evidence: Evidence) -> None:
    oracle = args.oracle_bin.resolve()
    artifact = args.artifact.resolve()
    record = evidence.base("full-snapshot-parity")
    preflight_before = machine_state()
    baseline_dir = evidence.results_root / "parity"
    baseline_content = baseline_dir / "sql-owned.content-v1.jsonl"
    baseline_lookups = baseline_dir / "sql-owned.lookups-v1.jsonl"
    try:
        verify_no_candidate_provider(evidence.candidate_worktree)
        verify_baseline_file(
            baseline_content,
            BASELINE_CONTENT_SHA256,
            BASELINE_CONTENT_BYTES,
            BASELINE_CONTENT_RECORDS,
        )
        verify_baseline_file(
            baseline_lookups,
            BASELINE_LOOKUP_SHA256,
            BASELINE_LOOKUP_BYTES,
            BASELINE_LOOKUP_RECORDS,
        )
    except (OSError, RuntimeError) as error:
        record.update(
            {
                "status": "fail",
                "command": None,
                "machine_state_before": preflight_before,
                "machine_state_after": machine_state(),
                "error": f"parity preflight failed: {error}",
                "concurrent_readers": 4,
                "fallback_probe": "sources-hidden-with-bwrap-before-open",
            }
        )
        evidence.append(record)
        raise
    output_dir = baseline_dir / f"{evidence.backend}.{evidence.candidate_commit[:7]}"
    try:
        output_dir.mkdir(parents=True, exist_ok=True)
    except OSError as error:
        record.update(
            {
                "status": "fail",
                "command": None,
                "machine_state_before": preflight_before,
                "machine_state_after": machine_state(),
                "error": f"failed to prepare parity output directory: {error}",
                "concurrent_readers": 4,
                "fallback_probe": "sources-hidden-with-bwrap-before-open",
            }
        )
        evidence.append(record)
        raise
    pairs = [
        (
            output_dir / f"reader-{index}.content-v1.jsonl",
            output_dir / f"reader-{index}.lookups-v1.jsonl",
        )
        for index in range(5)
    ]
    before = machine_state()
    commands: list[list[str]] = []
    processes: list[subprocess.Popen[str]] = []
    try:
        commands = [
            isolated_oracle_command(
                oracle, artifact, content, lookups, evidence.provider
            )
            for content, lookups in pairs
        ]
        first = subprocess.run(
            commands[0], text=True, capture_output=True, check=False
        )
        (output_dir / "reader-0.stdout.log").write_text(first.stdout)
        (output_dir / "reader-0.stderr.log").write_text(first.stderr)
        processes = [
            subprocess.Popen(
                command,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
            )
            for command in commands[1:]
        ]
        concurrent = [
            process.communicate() + (process.returncode,) for process in processes
        ]
    except (OSError, RuntimeError) as error:
        for process in processes:
            if process.poll() is None:
                process.terminate()
        for process in processes:
            try:
                process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
        record.update(
            {
                "status": "fail",
                "command": commands[0] if commands else None,
                "machine_state_before": before,
                "machine_state_after": machine_state(),
                "error": f"failed to launch parity command: {error}",
                "concurrent_readers": 4,
                "fallback_probe": "sources-hidden-with-bwrap-before-open",
            }
        )
        evidence.append(record)
        raise
    for index, (stdout, stderr, _status) in enumerate(concurrent, start=1):
        (output_dir / f"reader-{index}.stdout.log").write_text(stdout)
        (output_dir / f"reader-{index}.stderr.log").write_text(stderr)
    after = machine_state()
    statuses = [first.returncode, *(row[2] for row in concurrent)]
    files_match = all(
        filecmp.cmp(content, baseline_content, shallow=False)
        and filecmp.cmp(lookups, baseline_lookups, shallow=False)
        for content, lookups in pairs
        if content.is_file() and lookups.is_file()
    ) and all(content.is_file() and lookups.is_file() for content, lookups in pairs)
    record.update(
        {
            "status": "pass" if all(status == 0 for status in statuses) and files_match else "fail",
            "command": commands[0],
            "machine_state_before": before,
            "machine_state_after": after,
            "exit_statuses": statuses,
            "concurrent_readers": 4,
            "fallback_probe": "sources-hidden-with-bwrap-before-open",
            "content_sha256": sha256(pairs[0][0]) if pairs[0][0].is_file() else None,
            "lookup_sha256": sha256(pairs[0][1]) if pairs[0][1].is_file() else None,
            "content_bytes": pairs[0][0].stat().st_size if pairs[0][0].is_file() else None,
            "lookup_bytes": pairs[0][1].stat().st_size if pairs[0][1].is_file() else None,
            "content_records": count_lines(pairs[0][0]) if pairs[0][0].is_file() else None,
            "lookup_records": count_lines(pairs[0][1]) if pairs[0][1].is_file() else None,
        }
    )
    evidence.append(record)
    if record["status"] != "pass":
        raise RuntimeError("candidate parity or concurrent no-fallback probe failed")


def parser() -> argparse.ArgumentParser:
    root = Path(__file__).resolve().parents[1]
    result = argparse.ArgumentParser()
    result.add_argument("--repo", type=Path, default=root)
    result.add_argument("--results-root", type=Path, required=True)
    result.add_argument("--raw-results", type=Path)
    result.add_argument("--candidate-worktree", type=Path, required=True)
    result.add_argument("--backend", required=True)
    subparsers = result.add_subparsers(dest="action", required=True)

    allocation_parser = subparsers.add_parser("allocation")
    allocation_parser.add_argument("--runs", type=int, default=3)
    allocation_parser.add_argument("--warm-path", action="append", default=[])
    allocation_parser.add_argument("command", nargs=argparse.REMAINDER)

    multi_parser = subparsers.add_parser("multi-reader")
    multi_parser.add_argument("--candidate-bin", type=Path, required=True)
    multi_parser.add_argument("--artifact", type=Path, required=True)
    multi_parser.add_argument("--runs", type=int, default=3)
    multi_parser.add_argument("--iterations", type=int, default=20_000)
    multi_parser.add_argument("--hold-ms", type=int, default=10_000)

    parity_parser = subparsers.add_parser("parity")
    parity_parser.add_argument("--oracle-bin", type=Path, required=True)
    parity_parser.add_argument("--artifact", type=Path, required=True)
    return result


def main() -> int:
    args = parser().parse_args()
    if args.action == "allocation" and args.command[:1] == ["--"]:
        args.command = args.command[1:]
    try:
        evidence = Evidence(args)
        if args.action == "allocation":
            allocation(args, evidence)
        elif args.action == "multi-reader":
            multi_reader(args, evidence)
        elif args.action == "parity":
            parity(args, evidence)
        else:
            raise AssertionError(args.action)
    except (OSError, RuntimeError, ValueError, json.JSONDecodeError) as error:
        print(f"candidate benchmark orchestration failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
