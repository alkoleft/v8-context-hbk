#!/usr/bin/env python3
"""Verify S83 catalog/resolver parity for an immutable mapped candidate.

This is a correctness gate, not a performance benchmark.  It runs one
sequential replay and four concurrent replays, waits for every writer to exit,
then compares each complete JSONL transcript byte-for-byte with the frozen
owned baseline.  Candidate processes cannot see the source HBK or SQLite
provider at any point in their sandbox.
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
import stat
import struct
import subprocess
import sys
import time
from pathlib import Path
from typing import Any, Sequence


PROTOCOL_VERSION = "hbk-s83-semantic-parity-v1"
DATASET = "shcntx_ru-8.3.27.1859-schema16-extraction11"
PLATFORM_VERSION = "8.3.27.1859"
PROVIDER_SCHEMA_VERSION = 16
EXTRACTION_SCHEMA_VERSION = 11

HBK_PATH = Path("/opt/1cv8/x86_64/8.3.27.1859/shcntx_ru.hbk")
HBK_SIZE = 40_744_845
HBK_SHA256 = "5bdf0b3ed89932572c012faddc4d05ebfa2986595cf2849b6eb6e5e65a9a4d48"
PROVIDER_RELATIVE_PATH = Path(
    "target/snapshot-materialization/"
    "shcntx_ru.8.3.27.1859.schema16.release.sqlite"
)
RESULTS_RELATIVE_PATH = Path("target/hbk-zero-copy-experiment-8.3.27.1859")
PROVIDER_SIZE = 204_288_000
PROVIDER_SHA256 = (
    "55c2e09971712a13a49cbcf5889f203d7a9dfcec22aa0d333247ae722f6f0fab"
)

OWNED_TRANSCRIPT_COMMIT = "fc52ed2c850303f4f4c71e3ce96e64b335a50f95"
OWNED_MANIFEST_NAME = "catalog-resolver-query-manifest-v1.json"
OWNED_MANIFEST_SIZE = 9_060_281
OWNED_MANIFEST_LINES = 1
OWNED_MANIFEST_SHA256 = (
    "420463926ae586c95bd83354dfc4f1c9f0d3457134239dad0fa6a39bfeb1a203"
)
OWNED_TRANSCRIPT_NAME = "catalog-resolver-transcript-v1.jsonl"
OWNED_TRANSCRIPT_SIZE = 769_824_709
OWNED_TRANSCRIPT_LINES = 742_872
OWNED_TRANSCRIPT_SHA256 = (
    "1fe7f166caad8e8573b809a97f7104caf85301370f1d34017376bc82ee893a29"
)

SAFE_LABEL = re.compile(r"^[A-Za-z0-9._-]+$")
HEX_SHA256 = re.compile(r"^[0-9a-f]{64}$")
HEX_COMMIT = re.compile(r"^[0-9a-f]{40}$")
ARTIFACT_CONTRACTS: dict[str, dict[str, object]] = {
    "flat-h2": {
        "backends": (
            "s83-f0-semantic",
            "s83-d1-semantic",
            "s83-p1-semantic",
        ),
        "format_version": "not-separate",
        "layout_version": 2,
        "layout_flags": 0,
        "section_count": 63,
    },
    "flat-l1": {
        "backends": ("s83-l1-semantic",),
        "format_version": "not-separate",
        "layout_version": 3,
        "layout_flags": 1,
        "section_count": 63,
    },
    "flat-i1": {
        "backends": ("s83-i1-semantic",),
        "format_version": "not-separate",
        "layout_version": 3,
        "layout_flags": 1,
        "section_count": 64,
    },
    "flat-r1": {
        "backends": ("s83-r1-semantic",),
        "format_version": "not-separate",
        "layout_version": 1,
        "layout_flags": 1,
        "section_count": 71,
    },
    "rkyv-a0": {
        "backends": ("s83-a0-semantic",),
        "format_version": "1",
        "layout_version": 1,
        "layout_flags": "not-applicable",
        "section_count": "archive-root",
    },
}


def run_text(command: Sequence[str], cwd: Path | None = None) -> str:
    return subprocess.check_output(command, cwd=cwd, text=True).strip()


def git_text(worktree: Path, *args: str) -> str:
    return run_text(("git", "-C", str(worktree), *args))


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(8 * 1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def count_lines(path: Path) -> int:
    with path.open("rb") as stream:
        return sum(1 for _ in stream)


def absolute_without_resolving(path: Path) -> Path:
    return Path(os.path.abspath(path))


def reject_symlink_components(path: Path, root: Path) -> None:
    path = absolute_without_resolving(path)
    root = absolute_without_resolving(root)
    try:
        relative = path.relative_to(root)
    except ValueError as error:
        raise RuntimeError(f"path escapes fixed root {root}: {path}") from error
    current = root
    if current.is_symlink():
        raise RuntimeError(f"fixed root must not be a symlink: {current}")
    for component in relative.parts:
        current /= component
        if current.is_symlink():
            raise RuntimeError(f"evidence path component is a symlink: {current}")


def verified_results_subdirectory(
    results_root: Path, name: str, *, create: bool = False
) -> Path:
    path = results_root / name
    reject_symlink_components(path, results_root)
    if create:
        path.mkdir(parents=False, exist_ok=True)
    if not path.is_dir():
        raise RuntimeError(f"missing results subdirectory: {path}")
    if path.resolve() != path:
        raise RuntimeError(f"results subdirectory is not canonical: {path}")
    return path


def artifact_header_metadata(path: Path) -> dict[str, Any]:
    with path.open("rb") as stream:
        header = stream.read(512)
    if len(header) < 216:
        raise RuntimeError(f"candidate artifact header is truncated: {path}")

    magic = header[:8]
    if magic in (b"HBKFH2\0\0", b"HBKFI1\0\0", b"HBKFR1\0\0"):
        layout_version = struct.unpack_from("<I", header, 8)[0]
        layout_flags = struct.unpack_from("<I", header, 20)[0]
        section_count = struct.unpack_from("<I", header, 24)[0]
        kind_by_identity = {
            (b"HBKFH2\0\0", 2, 0, 63): "flat-h2",
            (b"HBKFH2\0\0", 3, 1, 63): "flat-l1",
            (b"HBKFI1\0\0", 3, 1, 64): "flat-i1",
            (b"HBKFR1\0\0", 1, 1, 71): "flat-r1",
        }
        kind = kind_by_identity.get(
            (magic, layout_version, layout_flags, section_count)
        )
        if kind is None:
            raise RuntimeError(
                "unsupported flat artifact identity "
                f"magic={magic!r}, layout={layout_version}, "
                f"flags={layout_flags}, sections={section_count}: {path}"
            )
        metadata = {
            "kind": kind,
            "format_version": "not-separate",
            "layout_version": layout_version,
            "layout_flags": layout_flags,
            "section_count": section_count,
            "extraction_schema_version": struct.unpack_from("<I", header, 12)[0],
            "provider_schema_version": struct.unpack_from("<I", header, 16)[0],
            "source_hbk_bytes": struct.unpack_from("<Q", header, 48)[0],
            "provider_sqlite_bytes": struct.unpack_from("<Q", header, 56)[0],
            "source_hbk_sha256": header[64:128].decode("ascii"),
            "provider_sqlite_sha256": header[128:192].decode("ascii"),
            "platform_version": header[200:216]
            .split(b"\0", 1)[0]
            .decode("ascii"),
        }
    elif magic == b"HBKRKYV\0":
        platform_length = header[112]
        metadata = {
            "kind": "rkyv-a0",
            "format_version": str(struct.unpack_from("<I", header, 8)[0]),
            "layout_version": struct.unpack_from("<I", header, 12)[0],
            "layout_flags": "not-applicable",
            "section_count": "archive-root",
            "provider_schema_version": struct.unpack_from("<I", header, 16)[0],
            "extraction_schema_version": struct.unpack_from("<I", header, 20)[0],
            "source_hbk_sha256": header[48:80].hex(),
            "platform_version": header[
                113 : 113 + platform_length
            ].decode("ascii"),
            "provider_sqlite_sha256": header[160:192].hex(),
            "provider_sqlite_bytes": struct.unpack_from("<Q", header, 192)[0],
            "source_hbk_bytes": struct.unpack_from("<Q", header, 200)[0],
        }
    else:
        raise RuntimeError(f"unsupported candidate artifact magic {magic!r}: {path}")

    expected = {
        "provider_schema_version": PROVIDER_SCHEMA_VERSION,
        "extraction_schema_version": EXTRACTION_SCHEMA_VERSION,
        "source_hbk_bytes": HBK_SIZE,
        "provider_sqlite_bytes": PROVIDER_SIZE,
        "source_hbk_sha256": HBK_SHA256,
        "provider_sqlite_sha256": PROVIDER_SHA256,
        "platform_version": PLATFORM_VERSION,
    }
    for field, expected_value in expected.items():
        if metadata[field] != expected_value:
            raise RuntimeError(
                f"candidate artifact header {field} mismatch: "
                f"expected {expected_value!r}, got {metadata[field]!r}"
            )
    return metadata


def verify_file(
    path: Path,
    *,
    expected_size: int,
    expected_sha256: str,
    expected_lines: int | None = None,
    expected_last_version: str | None = None,
) -> None:
    if not path.is_file():
        raise RuntimeError(f"missing frozen input: {path}")
    actual_size = path.stat().st_size
    if actual_size != expected_size:
        raise RuntimeError(
            f"size mismatch for {path}: expected {expected_size}, got {actual_size}"
        )
    actual_sha256 = sha256(path)
    if actual_sha256 != expected_sha256:
        raise RuntimeError(
            f"SHA-256 mismatch for {path}: "
            f"expected {expected_sha256}, got {actual_sha256}"
        )
    if expected_lines is not None:
        actual_lines = count_lines(path)
        if actual_lines != expected_lines:
            raise RuntimeError(
                f"line-count mismatch for {path}: "
                f"expected {expected_lines}, got {actual_lines}"
            )
    if expected_last_version is not None:
        with path.open("rb") as stream:
            stream.seek(-1, os.SEEK_END)
            if stream.read(1) != b"\n":
                raise RuntimeError(f"canonical JSONL lacks final newline: {path}")
        last_line = subprocess.check_output(("tail", "-n", "1", str(path)))
        try:
            value = json.loads(last_line)
        except json.JSONDecodeError as error:
            raise RuntimeError(f"final JSONL row is invalid in {path}: {error}") from error
        if value.get("version") != expected_last_version:
            raise RuntimeError(
                f"final JSONL version mismatch for {path}: "
                f"expected {expected_last_version!r}, got {value.get('version')!r}"
            )


def machine_state() -> dict[str, Any]:
    load_one, load_five, load_fifteen, scheduler, _last_pid = (
        Path("/proc/loadavg").read_text(encoding="ascii").split()
    )
    runnable, total = scheduler.split("/", 1)
    memory: dict[str, int] = {}
    wanted = {
        "MemAvailable": "available_kib",
        "MemFree": "free_kib",
        "Cached": "cached_kib",
        "Dirty": "dirty_kib",
        "SwapFree": "swap_free_kib",
    }
    for line in Path("/proc/meminfo").read_text(encoding="ascii").splitlines():
        name, value, *_unit = line.replace(":", "").split()
        if name in wanted:
            memory[wanted[name]] = int(value)
    return {
        "captured_unix_ns": str(time.time_ns()),
        "load_average": {
            "one_minute": float(load_one),
            "five_minutes": float(load_five),
            "fifteen_minutes": float(load_fifteen),
        },
        "scheduler": {
            "runnable_tasks": int(runnable),
            "total_tasks": int(total),
        },
        "memory": memory,
    }


def append_record(
    results_root: Path, path: Path, record: dict[str, Any]
) -> None:
    requested_parent = results_root / "results"
    if requested_parent.is_symlink():
        raise RuntimeError(
            f"raw evidence directory must not be a symlink: {requested_parent}"
        )
    allowed_parent = requested_parent.resolve()
    if not allowed_parent.is_relative_to(results_root):
        raise RuntimeError(
            f"raw evidence directory escapes results root: {allowed_parent}"
        )
    allowed_parent.mkdir(parents=True, exist_ok=True)
    if path.parent.resolve() != allowed_parent:
        raise RuntimeError(
            f"raw evidence must stay directly under {allowed_parent}: {path}"
        )
    if path.is_symlink():
        raise RuntimeError(f"raw evidence path must not be a symlink: {path}")
    if path.exists() and not path.is_file():
        raise RuntimeError(f"raw evidence path is not a regular file: {path}")
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a", encoding="utf-8") as stream:
        json.dump(record, stream, ensure_ascii=False, separators=(",", ":"))
        stream.write("\n")
        stream.flush()
        os.fsync(stream.fileno())
    print(json.dumps(record, ensure_ascii=False, separators=(",", ":")))


def verify_candidate_worktree(
    candidate_worktree: Path, expected_commit: str
) -> tuple[str, str]:
    if git_text(candidate_worktree, "status", "--porcelain"):
        raise RuntimeError(f"candidate worktree is dirty: {candidate_worktree}")
    actual_commit = git_text(candidate_worktree, "rev-parse", "HEAD")
    if actual_commit != expected_commit:
        raise RuntimeError(
            f"candidate commit mismatch: expected {expected_commit}, got {actual_commit}"
        )
    branch = git_text(candidate_worktree, "branch", "--show-current")
    return actual_commit, branch


def verify_driver_worktree(repo: Path) -> tuple[str, str]:
    dirty = git_text(repo, "status", "--porcelain")
    if dirty:
        raise RuntimeError(
            "semantic parity driver worktree is dirty; evidence would not "
            f"reproduce from the recorded commit:\n{dirty}"
        )
    return (
        git_text(repo, "rev-parse", "HEAD"),
        git_text(repo, "branch", "--show-current"),
    )


def verify_no_candidate_provider(candidate_worktree: Path) -> None:
    candidate_provider = candidate_worktree / PROVIDER_RELATIVE_PATH
    if candidate_provider.exists():
        raise RuntimeError(
            "candidate worktree contains the fallback SQLite provider path: "
            f"{candidate_provider}"
        )


def isolated_command(
    *,
    binary: Path,
    artifact: Path,
    slot_lock: Path,
    manifest: Path,
    transcript: Path,
) -> list[str]:
    bwrap = shutil.which("bwrap")
    if bwrap is None:
        raise RuntimeError("bwrap is required for the source-hidden parity probe")

    # Create only the destination ancestors needed by explicit binds.  No host
    # /home or /opt tree is mounted, so neither the S83 sources nor alternate
    # copies elsewhere in those trees are discoverable by the candidate.
    parents: set[Path] = {Path("/etc")}
    for path in (binary, artifact, slot_lock, manifest, transcript.parent):
        current = path if path.is_dir() else path.parent
        while current != Path("/"):
            parents.add(current)
            current = current.parent

    command = [
        bwrap,
        "--die-with-parent",
        "--unshare-all",
        "--new-session",
        "--clearenv",
        "--setenv",
        "PATH",
        "/usr/bin",
        "--setenv",
        "RUST_BACKTRACE",
        "0",
        "--dev",
        "/dev",
        "--proc",
        "/proc",
        "--tmpfs",
        "/tmp",
    ]
    for parent in sorted(parents, key=lambda value: len(value.parts)):
        command.extend(("--dir", str(parent)))
    for runtime_root in (Path("/usr"), Path("/lib"), Path("/lib64")):
        if runtime_root.exists():
            command.extend(("--ro-bind", str(runtime_root), str(runtime_root)))
    loader_cache = Path("/etc/ld.so.cache")
    if loader_cache.is_file():
        command.extend(("--ro-bind", str(loader_cache), str(loader_cache)))
    command.extend(
        [
            "--ro-bind",
            str(binary),
            str(binary),
            "--ro-bind",
            str(artifact),
            str(artifact),
            "--bind",
            str(slot_lock),
            str(slot_lock),
            "--ro-bind",
            str(manifest),
            str(manifest),
            "--bind",
            str(transcript.parent),
            str(transcript.parent),
            "--chdir",
            "/",
            "--",
            str(binary),
            str(artifact),
            str(manifest),
            str(transcript),
        ]
    )
    return command


def first_diff_offset(left: Path, right: Path) -> int | None:
    offset = 0
    with left.open("rb") as left_stream, right.open("rb") as right_stream:
        while True:
            left_chunk = left_stream.read(1024 * 1024)
            right_chunk = right_stream.read(1024 * 1024)
            if left_chunk == right_chunk:
                if not left_chunk:
                    return None
                offset += len(left_chunk)
                continue
            common = min(len(left_chunk), len(right_chunk))
            for index in range(common):
                if left_chunk[index] != right_chunk[index]:
                    return offset + index
            return offset + common


def stderr_tail(value: str, limit: int = 8_000) -> str:
    return value[-limit:]


def captured_machine_state() -> dict[str, Any]:
    try:
        return machine_state()
    except (OSError, ValueError) as error:
        return {"capture_error": str(error)}


def parser() -> argparse.ArgumentParser:
    root = Path(__file__).resolve().parents[1]
    result = argparse.ArgumentParser()
    result.add_argument("--repo", type=Path, default=root)
    result.add_argument("--results-root", type=Path, required=True)
    result.add_argument("--candidate-worktree", type=Path, required=True)
    result.add_argument("--candidate-commit", required=True)
    result.add_argument("--backend", required=True)
    result.add_argument("--run-id", required=True)
    result.add_argument("--candidate-bin", type=Path, required=True)
    result.add_argument("--candidate-bin-sha256", required=True)
    result.add_argument("--artifact", type=Path, required=True)
    result.add_argument("--artifact-sha256", required=True)
    result.add_argument("--artifact-size", type=int, required=True)
    result.add_argument("--artifact-mode", default="0444")
    result.add_argument(
        "--artifact-kind", choices=sorted(ARTIFACT_CONTRACTS), required=True
    )
    result.add_argument("--slot-lock", type=Path, required=True)
    return result


def execute(args: argparse.Namespace) -> None:
    repo = args.repo.resolve()
    results_root_input = absolute_without_resolving(args.results_root)
    results_root = results_root_input.resolve()
    candidate_worktree = args.candidate_worktree.resolve()
    binary_is_symlink = args.candidate_bin.is_symlink()
    binary = args.candidate_bin.resolve()
    artifact_input = absolute_without_resolving(args.artifact)
    artifact_is_symlink = artifact_input.is_symlink()
    artifact = artifact_input.resolve()
    slot_lock_input = absolute_without_resolving(args.slot_lock)
    slot_lock_is_symlink = slot_lock_input.is_symlink()
    slot_lock = slot_lock_input.resolve()
    provider = repo / PROVIDER_RELATIVE_PATH
    manifest = (
        results_root
        / "semantic-parity"
        / f"owned-{OWNED_TRANSCRIPT_COMMIT[:7]}"
        / OWNED_MANIFEST_NAME
    )
    baseline_transcript = manifest.with_name(OWNED_TRANSCRIPT_NAME)
    safe_backend = args.backend if SAFE_LABEL.fullmatch(args.backend) else "invalid"
    safe_commit = (
        args.candidate_commit[:7]
        if HEX_COMMIT.fullmatch(args.candidate_commit)
        else "invalid"
    )
    raw_results = (
        results_root
        / "results"
        / f"raw-semantic-{safe_backend}-{safe_commit}.jsonl"
    )
    record: dict[str, Any] = {
        "protocol_version": PROTOCOL_VERSION,
        "status": "fail",
        "phase": "preflight",
        "backend": args.backend,
        "run_id": args.run_id,
        "dataset": DATASET,
        "platform_version": PLATFORM_VERSION,
        "provider_schema_version": PROVIDER_SCHEMA_VERSION,
        "extraction_schema_version": EXTRACTION_SCHEMA_VERSION,
        "owned_transcript_commit": OWNED_TRANSCRIPT_COMMIT,
        "driver_commit": None,
        "driver_branch": None,
        "driver_worktree_clean": False,
        "candidate_commit": args.candidate_commit,
        "candidate_branch": None,
        "binary_path": str(binary),
        "binary_sha256": None,
        "artifact_path": str(artifact),
        "artifact_sha256": None,
        "artifact_size": None,
        "artifact_mode": None,
        "artifact_kind": args.artifact_kind,
        "artifact_format_version": None,
        "artifact_layout_version": None,
        "artifact_layout_flags": None,
        "artifact_section_count": None,
        "artifact_header": None,
        "slot_lock_path": str(slot_lock),
        "slot_lock_mode": None,
        "manifest_path": str(manifest),
        "manifest_sha256": OWNED_MANIFEST_SHA256,
        "manifest_size": OWNED_MANIFEST_SIZE,
        "baseline_transcript_path": str(baseline_transcript),
        "expected_transcript_sha256": OWNED_TRANSCRIPT_SHA256,
        "expected_transcript_size": OWNED_TRANSCRIPT_SIZE,
        "expected_transcript_records": OWNED_TRANSCRIPT_LINES,
        "concurrent_readers": 4,
        "fallback_probe": "sources-hidden-before-process-and-through-replay",
        "source_hbk_sha256": HBK_SHA256,
        "sqlite_sha256": PROVIDER_SHA256,
        "host": None,
        "machine_state_before": captured_machine_state(),
    }

    output_dir: Path | None = None
    processes: list[subprocess.Popen[str]] = []
    try:
        for label_name, label in (
            ("backend", args.backend),
            ("run-id", args.run_id),
        ):
            if not SAFE_LABEL.fullmatch(label):
                raise RuntimeError(f"unsafe {label_name} label: {label!r}")
        if not HEX_COMMIT.fullmatch(args.candidate_commit):
            raise RuntimeError(
                f"candidate commit must be a full lowercase SHA: {args.candidate_commit!r}"
            )
        for value_name, value in (
            ("candidate-bin-sha256", args.candidate_bin_sha256),
            ("artifact-sha256", args.artifact_sha256),
        ):
            if not HEX_SHA256.fullmatch(value):
                raise RuntimeError(
                    f"{value_name} must be a lowercase SHA-256: {value!r}"
                )
        expected_results_root = (repo / RESULTS_RELATIVE_PATH).resolve()
        expected_results_input = absolute_without_resolving(
            repo / RESULTS_RELATIVE_PATH
        )
        reject_symlink_components(expected_results_input, repo)
        if results_root_input != expected_results_input:
            raise RuntimeError(
                f"results root path mismatch: expected {expected_results_input}, "
                f"got {results_root_input}"
            )
        if results_root != expected_results_root:
            raise RuntimeError(
                f"results root mismatch: expected {expected_results_root}, "
                f"got {results_root}"
            )
        candidates_root = verified_results_subdirectory(
            results_root, "candidates"
        )
        semantic_root = verified_results_subdirectory(
            results_root, "semantic-parity"
        )
        verified_results_subdirectory(results_root, "results", create=True)
        owned_root = semantic_root / f"owned-{OWNED_TRANSCRIPT_COMMIT[:7]}"
        reject_symlink_components(owned_root, semantic_root)
        if not owned_root.is_dir() or owned_root.resolve() != owned_root:
            raise RuntimeError(f"invalid owned semantic baseline directory: {owned_root}")
        reject_symlink_components(manifest, semantic_root)
        reject_symlink_components(baseline_transcript, semantic_root)
        if artifact_is_symlink:
            raise RuntimeError(
                f"candidate artifact must not be a symlink: {args.artifact}"
            )
        if slot_lock_is_symlink:
            raise RuntimeError(f"slot lock must not be a symlink: {args.slot_lock}")
        if binary_is_symlink:
            raise RuntimeError(
                f"candidate binary must not be a symlink: {args.candidate_bin}"
            )

        driver_commit, driver_branch = verify_driver_worktree(repo)
        record.update(
            {
                "driver_commit": driver_commit,
                "driver_branch": driver_branch,
                "driver_worktree_clean": True,
                "host": {
                    "kernel": f"{platform.system()} {platform.release()}",
                    "architecture": platform.machine(),
                    "rustc": run_text(("rustc", "--version")),
                    "cargo": run_text(("cargo", "--version")),
                },
            }
        )
        candidate_commit, candidate_branch = verify_candidate_worktree(
            candidate_worktree, args.candidate_commit
        )
        record["candidate_commit"] = candidate_commit
        record["candidate_branch"] = candidate_branch
        verify_no_candidate_provider(candidate_worktree)
        verify_file(
            HBK_PATH, expected_size=HBK_SIZE, expected_sha256=HBK_SHA256
        )
        verify_file(
            provider,
            expected_size=PROVIDER_SIZE,
            expected_sha256=PROVIDER_SHA256,
        )
        verify_file(
            manifest,
            expected_size=OWNED_MANIFEST_SIZE,
            expected_sha256=OWNED_MANIFEST_SHA256,
            expected_lines=OWNED_MANIFEST_LINES,
            expected_last_version="catalog-resolver-query-manifest-v1",
        )
        verify_file(
            baseline_transcript,
            expected_size=OWNED_TRANSCRIPT_SIZE,
            expected_sha256=OWNED_TRANSCRIPT_SHA256,
            expected_lines=OWNED_TRANSCRIPT_LINES,
            expected_last_version="catalog-resolver-transcript-v1",
        )
        if not os.access(binary, os.X_OK):
            raise RuntimeError(f"candidate binary is not executable: {binary}")
        expected_binary_parent = (
            candidate_worktree / "target" / "release" / "examples"
        ).resolve()
        if binary.parent != expected_binary_parent:
            raise RuntimeError(
                "candidate binary must be the worktree release example: "
                f"expected parent {expected_binary_parent}, got {binary}"
            )
        actual_binary_sha = sha256(binary)
        record["binary_sha256"] = actual_binary_sha
        if actual_binary_sha != args.candidate_bin_sha256:
            raise RuntimeError(
                "candidate binary SHA-256 mismatch: "
                f"expected {args.candidate_bin_sha256}, got {actual_binary_sha}"
            )
        if not artifact.is_file():
            raise RuntimeError(f"candidate artifact is missing: {artifact}")
        reject_symlink_components(artifact_input, candidates_root)
        reject_symlink_components(slot_lock_input, candidates_root)
        if artifact_input.parent.parent != candidates_root:
            raise RuntimeError(
                "candidate artifact must be one generation directory below "
                f"{candidates_root}: {artifact_input}"
            )
        if artifact != artifact_input:
            raise RuntimeError(
                f"candidate artifact path is not canonical: {artifact_input}"
            )
        if not slot_lock.is_file():
            raise RuntimeError(f"candidate slot lock is missing: {slot_lock}")
        if slot_lock_input.parent != artifact_input.parent:
            raise RuntimeError(
                "candidate slot lock must be adjacent to the artifact: "
                f"{slot_lock_input} vs {artifact_input}"
            )
        expected_slot_lock = artifact_input.with_name(f"{artifact_input.name}.lock")
        if slot_lock_input != expected_slot_lock or slot_lock != slot_lock_input:
            raise RuntimeError(
                f"unexpected slot lock path: expected {expected_slot_lock}, "
                f"got {slot_lock_input}"
            )
        record["slot_lock_mode"] = (
            f"{stat.S_IMODE(slot_lock.stat().st_mode):04o}"
        )
        actual_artifact_size = artifact.stat().st_size
        actual_artifact_sha = sha256(artifact)
        actual_artifact_mode = stat.S_IMODE(artifact.stat().st_mode)
        header_metadata = artifact_header_metadata(artifact)
        record["artifact_header"] = header_metadata
        contract = ARTIFACT_CONTRACTS[args.artifact_kind]
        if header_metadata["kind"] != args.artifact_kind:
            raise RuntimeError(
                "candidate artifact kind mismatch: "
                f"expected {args.artifact_kind!r}, "
                f"header has {header_metadata['kind']!r}"
            )
        allowed_backends = contract["backends"]
        if (
            not isinstance(allowed_backends, tuple)
            or args.backend not in allowed_backends
        ):
            raise RuntimeError(
                f"backend does not match {args.artifact_kind}: "
                f"expected one of {allowed_backends!r}, got {args.backend!r}"
            )
        for field in (
            "format_version",
            "layout_version",
            "layout_flags",
            "section_count",
        ):
            if header_metadata[field] != contract[field]:
                raise RuntimeError(
                    f"unsupported {args.artifact_kind} {field}: "
                    f"expected {contract[field]!r}, "
                    f"got {header_metadata[field]!r}"
                )
        record["artifact_format_version"] = header_metadata["format_version"]
        record["artifact_layout_version"] = header_metadata["layout_version"]
        record["artifact_layout_flags"] = header_metadata["layout_flags"]
        record["artifact_section_count"] = header_metadata["section_count"]
        expected_artifact_mode = int(args.artifact_mode, 8)
        record.update(
            {
                "artifact_sha256": actual_artifact_sha,
                "artifact_size": actual_artifact_size,
                "artifact_mode": f"{actual_artifact_mode:04o}",
            }
        )
        if actual_artifact_size != args.artifact_size:
            raise RuntimeError(
                "candidate artifact size mismatch: "
                f"expected {args.artifact_size}, got {actual_artifact_size}"
            )
        if actual_artifact_sha != args.artifact_sha256:
            raise RuntimeError(
                "candidate artifact SHA-256 mismatch: "
                f"expected {args.artifact_sha256}, got {actual_artifact_sha}"
            )
        if actual_artifact_mode != expected_artifact_mode:
            raise RuntimeError(
                "candidate artifact mode mismatch: "
                f"expected {expected_artifact_mode:04o}, got {actual_artifact_mode:04o}"
            )

        output_dir = (
            semantic_root
            / f"{args.backend}-{args.candidate_commit[:7]}-{args.run_id}"
        )
        reject_symlink_components(output_dir, semantic_root)
        output_dir.mkdir(parents=True, exist_ok=False)
        transcripts = [
            output_dir / "sequential.transcript.jsonl",
            *[
                output_dir / f"concurrent-{worker}.transcript.jsonl"
                for worker in range(1, 5)
            ],
        ]
        commands = [
            isolated_command(
                binary=binary,
                artifact=artifact,
                slot_lock=slot_lock,
                manifest=manifest,
                transcript=transcript,
            )
            for transcript in transcripts
        ]
        record["commands"] = commands
        record["phase"] = "replay"
        started = time.monotonic()
        sequential = subprocess.run(
            commands[0], text=True, capture_output=True, check=False
        )
        (output_dir / "sequential.stdout.log").write_text(
            sequential.stdout, encoding="utf-8"
        )
        (output_dir / "sequential.stderr.log").write_text(
            sequential.stderr, encoding="utf-8"
        )
        if sequential.returncode != 0:
            record.update(
                {
                    "exit_statuses": [sequential.returncode],
                    "stderr_tail": stderr_tail(sequential.stderr),
                }
            )
            raise RuntimeError(
                f"sequential semantic replay exited {sequential.returncode}"
            )

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
        for worker, (stdout, stderr, _status) in enumerate(concurrent, start=1):
            (output_dir / f"concurrent-{worker}.stdout.log").write_text(
                stdout, encoding="utf-8"
            )
            (output_dir / f"concurrent-{worker}.stderr.log").write_text(
                stderr, encoding="utf-8"
            )
        statuses = [sequential.returncode, *(row[2] for row in concurrent)]
        record["exit_statuses"] = statuses
        record["replay_elapsed_seconds"] = time.monotonic() - started
        if any(status != 0 for status in statuses):
            failing_stderr = next(
                (
                    stderr
                    for _stdout, stderr, status in concurrent
                    if status != 0
                ),
                "",
            )
            record["stderr_tail"] = stderr_tail(failing_stderr)
            raise RuntimeError(f"concurrent semantic replay exits: {statuses}")

        # All writers have exited.  Only now inspect, hash, or compare outputs.
        record["phase"] = "compare"
        output_rows: list[dict[str, Any]] = []
        for transcript in transcripts:
            verify_file(
                transcript,
                expected_size=OWNED_TRANSCRIPT_SIZE,
                expected_sha256=OWNED_TRANSCRIPT_SHA256,
                expected_lines=OWNED_TRANSCRIPT_LINES,
                expected_last_version="catalog-resolver-transcript-v1",
            )
            baseline_equal = filecmp.cmp(
                transcript, baseline_transcript, shallow=False
            )
            sequential_equal = filecmp.cmp(
                transcript, transcripts[0], shallow=False
            )
            output_rows.append(
                {
                    "path": str(transcript),
                    "sha256": OWNED_TRANSCRIPT_SHA256,
                    "size": OWNED_TRANSCRIPT_SIZE,
                    "records": OWNED_TRANSCRIPT_LINES,
                    "baseline_byte_equal": baseline_equal,
                    "sequential_byte_equal": sequential_equal,
                }
            )
            if not baseline_equal:
                record.update(
                    {
                        "mismatch_path": str(transcript),
                        "first_diff_offset": first_diff_offset(
                            baseline_transcript, transcript
                        ),
                    }
                )
                raise RuntimeError(
                    f"semantic transcript differs from owned baseline: {transcript}"
                )
            if not sequential_equal:
                record.update(
                    {
                        "mismatch_path": str(transcript),
                        "first_diff_offset": first_diff_offset(
                            transcripts[0], transcript
                        ),
                    }
                )
                raise RuntimeError(
                    f"concurrent semantic transcript differs: {transcript}"
                )

        record.update(
            {
                "status": "pass",
                "phase": "complete",
                "outputs": output_rows,
                "machine_state_after": captured_machine_state(),
            }
        )
    except (OSError, RuntimeError, ValueError, subprocess.SubprocessError) as error:
        for process in processes:
            if process.poll() is None:
                process.terminate()
        for process in processes:
            if process.poll() is None:
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
        record.update(
            {
                "status": "fail",
                "error": str(error),
                "machine_state_after": captured_machine_state(),
            }
        )
        append_record(results_root, raw_results, record)
        raise
    else:
        append_record(results_root, raw_results, record)


def main() -> int:
    args = parser().parse_args()
    try:
        execute(args)
    except (OSError, RuntimeError, ValueError, subprocess.SubprocessError) as error:
        print(f"semantic parity verification failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
