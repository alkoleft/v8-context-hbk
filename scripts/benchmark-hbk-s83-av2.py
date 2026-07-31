#!/usr/bin/env python3
"""Run the frozen S83-AV2 context/member-access evidence matrix.

The orchestration config is a strict JSON object with ``schema_version`` set to
``hbk-s83-av2-orchestration/v1``, a ``query_manifest`` artifact pointer, and an
exact ``backends`` registry for S83-H0/C0/F0/A0/L1/I1/D1/P1/R1.  Backend
commands are argv arrays; they are never passed through a shell and may use
``{mode}``, ``{operation}``, ``{context}``, ``{iterations}``, and
``{query_manifest}`` placeholders. Empty substituted argv elements are removed.

This script is orchestration and validation only.  The Rust harness emits the
versioned report/parity JSON; this driver refuses schema drift before evidence
is appended under the target results root.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import re
import socket
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable, Iterator, Sequence


MANIFEST_SCHEMA = "hbk-s83-av2-query-manifest/v1"
REPORT_SCHEMA = "hbk-s83-av2-benchmark/v1"
RAW_SCHEMA = "hbk-s83-av2-raw/v1"
PARITY_SCHEMA = "hbk-s83-av2-parity/v1"
SUMMARY_SCHEMA = "hbk-s83-av2-summary/v1"
PREFLIGHT_SMOKE_SCHEMA = "hbk-s83-av2-preflight-smoke/v1"
WORKLOAD_VERSION = "s83-av2-context-member-access/v1"
ORCHESTRATION_VERSION = "hbk-s83-av2-orchestration/v1"

DATASET = "shcntx_ru-8.3.27.1859-schema16-extraction11"
PLATFORM_VERSION = "8.3.27.1859"
SOURCE_LOCALE = "ru"
PROVIDER_SCHEMA_VERSION = 16
EXTRACTION_SCHEMA_VERSION = 11
HBK_BYTES = 40_744_845
HBK_SHA256 = "5bdf0b3ed89932572c012faddc4d05ebfa2986595cf2849b6eb6e5e65a9a4d48"
PROVIDER_BYTES = 204_288_000
PROVIDER_SHA256 = "55c2e09971712a13a49cbcf5889f203d7a9dfcec22aa0d333247ae722f6f0fab"

BACKENDS = (
    "S83-H0",
    "S83-C0",
    "S83-F0",
    "S83-A0",
    "S83-L1",
    "S83-I1",
    "S83-D1",
    "S83-P1",
    "S83-R1",
)
DECISION_ROLES = {
    "S83-H0": "baseline",
    "S83-C0": "control",
    "S83-F0": "candidate",
    "S83-A0": "candidate",
    "S83-L1": "candidate",
    "S83-I1": "candidate",
    "S83-D1": "candidate",
    "S83-P1": "candidate",
    "S83-R1": "candidate",
}
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
OPERATIONS = (
    "type_by_name",
    "property_by_owner_name_kind",
    "method_by_owner_name_kind",
    "callable_by_owner_name",
    "members_by_owner_availability_borrowed",
    "members_by_owner_availability_collect",
    "type_payload",
    "method_payload",
    "property_payload",
    "filtered_members_payload",
)
LOOKUP_OPERATIONS = frozenset(
    {
        "type_by_name",
        "property_by_owner_name_kind",
        "method_by_owner_name_kind",
        "callable_by_owner_name",
    }
)
ITERATION_OPERATIONS = frozenset({"members_by_owner_availability_borrowed"})
COMPACT_OPERATIONS = frozenset({"members_by_owner_availability_collect"})
PAYLOAD_OPERATIONS = frozenset(
    {"type_payload", "method_payload", "property_payload", "filtered_members_payload"}
)
CONTEXTUAL_OPERATIONS = frozenset(
    {
        "members_by_owner_availability_borrowed",
        "members_by_owner_availability_collect",
        "filtered_members_payload",
    }
)
OPERATION_TAG = {
    **{operation: "lookup" for operation in LOOKUP_OPERATIONS},
    "members_by_owner_availability_borrowed": "iteration",
    "members_by_owner_availability_collect": "compact_materialization",
    **{operation: "payload" for operation in PAYLOAD_OPERATIONS},
}
PHASE_ORDER = (
    "entry_to_ready",
    "anchor_resolution",
    "first_operation",
    "warmup",
    "steady_workload",
    "memory_sample",
)
DEFAULT_SAMPLES = 9
DEFAULT_POINT_LOOKUP_ITERATIONS = 100
DEFAULT_ENUMERATION_ITERATIONS = 1_000
DEFAULT_PARITY_ITERATIONS = 1
MAX_MANIFEST_BYTES = 64 * 1024 * 1024
MAX_PARITY_BYTES = 512 * 1024 * 1024
MAX_RAW_BYTES = 2 * 1024 * 1024 * 1024
OPERATION_CONTEXT_ROW_COUNT = sum(
    len(AVAILABILITY_CONTEXTS) if operation in CONTEXTUAL_OPERATIONS else 1
    for operation in OPERATIONS
)
PARITY_ROW_COUNT = len(BACKENDS) * len(AVAILABILITY_CONTEXTS)
PERFORMANCE_ROW_COUNT = OPERATION_CONTEXT_ROW_COUNT * len(BACKENDS) * len(CACHE_STANCES) * DEFAULT_SAMPLES
SMOKE_OPERATION = "members_by_owner_availability_borrowed"
SMOKE_CONTEXT = "thin_client"
SMOKE_ITERATIONS = 1
SMOKE_ROW_COUNT = len(BACKENDS)
U32_MAX = 2**32 - 1

HARNESS_PATHS = (
    "crates/syntax-helper-search/Cargo.toml",
    "crates/syntax-helper-search/examples/measure_hbk_s83_av2.rs",
    "scripts/benchmark-hbk-s83-av2.py",
    "scripts/summarize-hbk-s83-av2-results.py",
)
BACKEND_PATTERN = re.compile(r"^[A-Za-z0-9._-]+$")
FORBIDDEN_MODULE_CONTEXT_KEY = re.compile(r"module_?context_?kind", re.IGNORECASE)
FORBIDDEN_RANKING_KEY = re.compile(
    r"(^|_)(rank|score|winner|recommendation)(_|$)", re.IGNORECASE
)
FORBIDDEN_COMPACT_PAYLOAD_KEY = re.compile(
    r"(retained_.*(payload|string|dto)|compact_.*(payload|string|dto)|resolver_?dto)",
    re.IGNORECASE,
)

ORCHESTRATION_KEYS = frozenset({"schema_version", "query_manifest", "preflight_estimates", "backends"})
QUERY_MANIFEST_KEYS = frozenset(
    {
        "schema_version",
        "workload_version",
        "input_identity",
        "availability_contexts",
        "member_kinds",
        "empty_availability_rule",
        "module_context_filter_used",
        "types",
        "members",
        "lookup_queries",
        "fixed_misses",
        "anchors",
    }
)
ARTIFACT_KEYS = frozenset({"path", "bytes", "sha256"})
BACKEND_KEYS = frozenset(
    {"backend", "decision_role", "worktree", "command", "declared_files"}
)
PREFLIGHT_ESTIMATE_KEYS = frozenset(
    {
        "manifest_bytes",
        "max_parity_bytes",
        "raw_run_bytes",
        "operation_context_rows",
        "parity_rows",
        "performance_rows",
    }
)
REPORT_KEYS = frozenset(
    {
        "schema_version",
        "workload_version",
        "mode",
        "backend",
        "decision_role",
        "operation",
        "availability_context",
        "iterations",
        "module_context_filter_used",
        "empty_availability_rule",
        "input_identity",
        "manifest",
        "runtime_artifacts",
        "projection",
        "phase_order",
        "timings",
        "faults",
        "allocations",
        "memory",
        "counts",
        "checksum",
        "operation_data",
    }
)
PARITY_REPORT_KEYS = frozenset(
    {
        "schema_version",
        "workload_version",
        "mode",
        "backend",
        "decision_role",
        "availability_context",
        "module_context_filter_used",
        "empty_availability_rule",
        "input_identity",
        "manifest",
        "runtime_artifacts",
        "transcript",
    }
)
INPUT_IDENTITY_KEYS = frozenset(
    {
        "dataset",
        "platform_version",
        "source_locale",
        "provider_schema_version",
        "extraction_schema_version",
        "hbk",
        "provider",
    }
)
QUERY_INPUT_IDENTITY_KEYS = INPUT_IDENTITY_KEYS
MANIFEST_IDENTITY_KEYS = frozenset({"schema_version", "sha256", "bytes"})
QUERY_TYPE_KEYS = frozenset({"logical_id", "primary", "alias", "member_count"})
QUERY_MEMBER_KEYS = frozenset({"logical_id", "owner_logical_id", "kind", "primary", "alias"})
QUERY_LOOKUP_KEYS = frozenset({"type_names", "properties", "methods"})
QUERY_TYPE_LOOKUP_KEYS = frozenset({"logical_id", "query_name", "query_role"})
QUERY_MEMBER_LOOKUP_KEYS = frozenset(
    {"logical_id", "owner_logical_id", "kind", "query_name", "query_role"}
)
QUERY_MISSES_KEYS = frozenset({"type_name", "member_name", "callable_name"})
QUERY_ANCHORS_KEYS = frozenset(
    {
        "type_primary",
        "property_owner",
        "property_name",
        "method_owner",
        "method_name",
        "enumeration_owner",
    }
)
PROJECTION_KEYS = frozenset({"source", "compact"})
FAULT_KEYS = frozenset({"minor", "major"})
TIMING_PHASE_KEYS = frozenset(
    {"elapsed_ns", "average_ns", "ns_per_query", "ns_per_object", "count", "checksum"}
)
ALLOCATION_DELTA_KEYS = frozenset(
    {
        "allocation_calls",
        "reallocation_calls",
        "deallocation_calls",
        "allocated_bytes",
        "deallocated_bytes",
        "live_bytes_before",
        "live_bytes_after",
        "peak_live_bytes_before",
        "peak_live_bytes_after",
        "peak_live_bytes_growth",
    }
)
ALLOCATION_KEYS = frozenset({"enabled", *PHASE_ORDER})
MEMORY_KEYS = frozenset(
    {
        "before_kib",
        "live_kib",
        "after_drop_kib",
        "container_overhead_bytes",
        "logical_bytes",
        "capacity_bytes",
        "live_delta_bytes",
        "peak_live_delta_bytes",
        "post_drop_delta_bytes",
    }
)
PROCESS_MEMORY_KEYS = frozenset(
    {"rss_kib", "pss_kib", "private_kib", "anonymous_kib", "file_backed_kib"}
)
COUNT_KEYS = frozenset(
    {
        "query_count",
        "candidate_count",
        "object_count",
        "checksum_count",
        "property_count",
        "method_count",
        "event_count",
        "enum_value_count",
    }
)
LOOKUP_DATA_KEYS = frozenset({"tag", "query_count", "candidate_count", "miss_count"})
ITERATION_DATA_KEYS = frozenset(
    {
        "tag",
        "owner_count",
        "scanned_count",
        "returned_count",
        "universal_count",
        "explicit_count",
        "excluded_count",
        "property_count",
        "method_count",
        "event_count",
        "enum_value_count",
    }
)
COMPACT_DATA_KEYS = frozenset(
    {
        *ITERATION_DATA_KEYS,
        "locator_size",
        "total_len",
        "total_capacity",
        "logical_bytes",
        "allocated_bytes",
    }
)
PAYLOAD_DATA_KEYS = frozenset(
    {
        "tag",
        "input_count",
        "object_count",
        "string_bytes_touched",
        "canonical_payload_bytes_touched",
    }
)


class EvidenceError(RuntimeError):
    """An S83-AV2 evidence contract violation."""


@dataclass(frozen=True)
class Backend:
    backend: str
    decision_role: str
    worktree: Path
    command: tuple[str, ...]
    declared_files: tuple[Path, ...]
    declared_file_artifacts: tuple[dict[str, Any], ...]
    executable: Path
    executable_artifact: dict[str, Any]
    commit: str
    branch: str


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo", type=Path, default=Path.cwd())
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--results-root", type=Path, required=True)
    parser.add_argument("--samples", type=positive_int, default=DEFAULT_SAMPLES)
    parser.add_argument("--timeout-seconds", type=positive_int, default=900)
    parser.add_argument("--parity-iterations", type=positive_int, default=DEFAULT_PARITY_ITERATIONS)
    parser.add_argument("--lookup-iterations", type=positive_int, default=DEFAULT_POINT_LOOKUP_ITERATIONS)
    parser.add_argument("--payload-iterations", type=positive_int, default=DEFAULT_POINT_LOOKUP_ITERATIONS)
    parser.add_argument("--enumeration-iterations", type=positive_int, default=DEFAULT_ENUMERATION_ITERATIONS)
    return parser.parse_args(argv)


def positive_int(value: str) -> int:
    parsed = int(value)
    if parsed <= 0:
        raise argparse.ArgumentTypeError("value must be greater than zero")
    return parsed


def validate_frozen_run_parameters(args: argparse.Namespace) -> None:
    expected = {
        "samples": DEFAULT_SAMPLES,
        "parity_iterations": DEFAULT_PARITY_ITERATIONS,
        "lookup_iterations": DEFAULT_POINT_LOOKUP_ITERATIONS,
        "payload_iterations": DEFAULT_POINT_LOOKUP_ITERATIONS,
        "enumeration_iterations": DEFAULT_ENUMERATION_ITERATIONS,
    }
    for name, frozen_value in expected.items():
        if getattr(args, name) != frozen_value:
            raise EvidenceError(
                f"{name} must remain frozen at {frozen_value} for hbk-s83-av2/v1"
            )


def run_text(command: Sequence[str], cwd: Path | None = None) -> str:
    return subprocess.check_output(command, cwd=cwd, text=True).strip()


def git_text(worktree: Path, *args: str) -> str:
    return run_text(("git", "-C", str(worktree), *args))


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def canonical_json_bytes(value: Any) -> bytes:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")


def require_object(value: Any, path: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise EvidenceError(f"{path} must be an object")
    return value


def require_exact_keys(value: dict[str, Any], expected: frozenset[str], path: str) -> None:
    actual = set(value)
    if actual != expected:
        raise EvidenceError(
            f"{path} schema keys differ: missing={sorted(expected - actual)}, "
            f"unexpected={sorted(actual - expected)}"
        )


def require_integer(value: Any, path: str, *, positive: bool = False) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        raise EvidenceError(f"{path} must be an integer")
    if positive and value <= 0:
        raise EvidenceError(f"{path} must be greater than zero")
    if not positive and value < 0:
        raise EvidenceError(f"{path} must be non-negative")
    return value


def require_optional_number(value: Any, path: str) -> None:
    if value is None:
        return
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise EvidenceError(f"{path} must be numeric or null")
    if value < 0:
        raise EvidenceError(f"{path} must be non-negative")


def reject_forbidden_fields(value: Any, path: str = "") -> None:
    if isinstance(value, dict):
        for key, nested in value.items():
            key_text = str(key)
            nested_path = f"{path}.{key_text}" if path else key_text
            if key_text != "module_context_filter_used" and FORBIDDEN_MODULE_CONTEXT_KEY.search(key_text):
                raise EvidenceError(f"ModuleContextKind is forbidden at {nested_path}")
            if FORBIDDEN_RANKING_KEY.search(key_text):
                raise EvidenceError(f"ranking/selection field is forbidden at {nested_path}")
            if FORBIDDEN_COMPACT_PAYLOAD_KEY.search(key_text):
                raise EvidenceError(f"retained compact payload/string/DTO field is forbidden at {nested_path}")
            reject_forbidden_fields(nested, nested_path)
    elif isinstance(value, list):
        for index, nested in enumerate(value):
            reject_forbidden_fields(nested, f"{path}[{index}]")


def validate_artifact(value: Any, path: str, *, expected_bytes: int | None = None, expected_sha256: str | None = None) -> dict[str, Any]:
    artifact = require_object(value, path)
    require_exact_keys(artifact, ARTIFACT_KEYS, path)
    raw_path = artifact["path"]
    if not isinstance(raw_path, str) or not raw_path:
        raise EvidenceError(f"{path}.path must be a non-empty string")
    size = require_integer(artifact["bytes"], f"{path}.bytes", positive=True)
    digest = artifact["sha256"]
    if not isinstance(digest, str) or not re.fullmatch(r"[0-9a-f]{64}", digest):
        raise EvidenceError(f"{path}.sha256 must be a lowercase SHA-256")
    if expected_bytes is not None and size != expected_bytes:
        raise EvidenceError(f"{path}.bytes does not identify frozen S83")
    if expected_sha256 is not None and digest != expected_sha256:
        raise EvidenceError(f"{path}.sha256 does not identify frozen S83")
    return artifact


def resolve_path(worktree: Path, raw: str) -> Path:
    path = Path(raw)
    return path if path.is_absolute() else worktree / path


def artifact_identity(path: Path) -> dict[str, Any]:
    resolved = path.resolve()
    return {"path": str(resolved), "bytes": resolved.stat().st_size, "sha256": sha256_file(resolved)}


def resolve_executable(worktree: Path, raw: str) -> Path:
    candidate = resolve_path(worktree, raw).resolve()
    if not candidate.is_file():
        raise EvidenceError(f"backend executable must be a regular file: {candidate}")
    if not os.access(candidate, os.X_OK):
        raise EvidenceError(f"backend executable is not executable: {candidate}")
    return candidate


def source_projection_for(backend: str, operation: str) -> str:
    if operation in PAYLOAD_OPERATIONS:
        if backend in {"S83-H0", "S83-C0"}:
            return "owned-reference"
        if backend == "S83-A0":
            return "archived-view"
        if backend == "S83-R1":
            return "borrowed-range"
        return "decoded-value"
    if backend in {"S83-H0", "S83-C0"}:
        return "owned-id-slice"
    if backend == "S83-A0":
        return "archived-id-range"
    return "mapped-id-range"


def compact_projection_for(operation: str) -> str | None:
    return "av2-member-locator-u32" if operation in COMPACT_OPERATIONS else None


PROJECTION_REGISTRY = {
    backend: {
        operation: {
            "source": source_projection_for(backend, operation),
            "compact": compact_projection_for(operation),
        }
        for operation in OPERATIONS
    }
    for backend in BACKENDS
}


def operation_contexts(operation: str) -> tuple[str | None, ...]:
    return AVAILABILITY_CONTEXTS if operation in CONTEXTUAL_OPERATIONS else (None,)


def iterations_for(operation: str, lookup_iterations: int, payload_iterations: int, enumeration_iterations: int) -> int:
    if operation in LOOKUP_OPERATIONS:
        return lookup_iterations
    if operation in PAYLOAD_OPERATIONS:
        return payload_iterations
    return enumeration_iterations


def command_for(
    template: Sequence[str],
    mode: str,
    operation: str | None,
    context: str | None,
    iterations: int | None,
    query_manifest: Path | str,
) -> list[str]:
    context_value = "" if context is None else context
    replacements = {
        "{mode}": mode,
        "{operation}": "" if operation is None else operation,
        "{context}": context_value,
        "{iterations}": "" if iterations is None else str(iterations),
        "{query_manifest}": str(query_manifest),
    }
    command = list(template)
    for placeholder, value in replacements.items():
        command = [part.replace(placeholder, value) for part in command]
    leftovers = [part for part in command if re.search(r"\{(mode|operation|context|iterations|query_manifest)\}", part)]
    if leftovers:
        raise EvidenceError(f"unexpanded command placeholder: {leftovers}")
    return [part for part in command if part != ""]


def validate_query_input_identity(value: Any, path: str) -> None:
    identity = require_object(value, path)
    require_exact_keys(identity, QUERY_INPUT_IDENTITY_KEYS, path)
    identity_expected = {
        "dataset": DATASET,
        "platform_version": PLATFORM_VERSION,
        "source_locale": SOURCE_LOCALE,
        "provider_schema_version": PROVIDER_SCHEMA_VERSION,
        "extraction_schema_version": EXTRACTION_SCHEMA_VERSION,
    }
    for key, expected_value in identity_expected.items():
        if identity[key] != expected_value:
            raise EvidenceError(f"{path}.{key} does not identify frozen S83")
    validate_artifact(identity["hbk"], f"{path}.hbk", expected_bytes=HBK_BYTES, expected_sha256=HBK_SHA256)
    validate_artifact(identity["provider"], f"{path}.provider", expected_bytes=PROVIDER_BYTES, expected_sha256=PROVIDER_SHA256)


def validate_query_string(value: Any, path: str) -> None:
    if not isinstance(value, str) or not value:
        raise EvidenceError(f"{path} must be a non-empty string")


def validate_query_manifest_identity(value: dict[str, Any]) -> None:
    reject_forbidden_fields(value)
    expected = {
        "schema_version": MANIFEST_SCHEMA,
        "workload_version": WORKLOAD_VERSION,
        "availability_contexts": list(AVAILABILITY_CONTEXTS),
        "member_kinds": ["property", "method", "event", "enum_value"],
        "empty_availability_rule": "universal",
        "module_context_filter_used": False,
    }
    for key, expected_value in expected.items():
        if value.get(key) != expected_value:
            raise EvidenceError(f"query_manifest.{key} differs from frozen S83-AV2 contract")
    validate_query_input_identity(value["input_identity"], "query_manifest.input_identity")
    types = value["types"]
    if not isinstance(types, list) or not types:
        raise EvidenceError("query_manifest.types must contain all S83 platform types")
    seen_types: set[str] = set()
    for index, raw_type in enumerate(types):
        ty = require_object(raw_type, f"query_manifest.types[{index}]")
        require_exact_keys(ty, QUERY_TYPE_KEYS, f"query_manifest.types[{index}]")
        validate_query_string(ty["logical_id"], f"query_manifest.types[{index}].logical_id")
        validate_query_string(ty["primary"], f"query_manifest.types[{index}].primary")
        if ty["alias"] is not None:
            validate_query_string(ty["alias"], f"query_manifest.types[{index}].alias")
        require_integer(ty["member_count"], f"query_manifest.types[{index}].member_count")
        if ty["logical_id"] in seen_types:
            raise EvidenceError(f"duplicate type logical_id in query_manifest: {ty['logical_id']}")
        seen_types.add(ty["logical_id"])
    members = value["members"]
    if not isinstance(members, list) or not members:
        raise EvidenceError("query_manifest.members must contain all S83 type members")
    seen_members: set[str] = set()
    member_count_by_owner = {logical_id: 0 for logical_id in seen_types}
    for index, raw_member in enumerate(members):
        member = require_object(raw_member, f"query_manifest.members[{index}]")
        require_exact_keys(member, QUERY_MEMBER_KEYS, f"query_manifest.members[{index}]")
        for key in ("logical_id", "owner_logical_id", "kind", "primary"):
            validate_query_string(member[key], f"query_manifest.members[{index}].{key}")
        if member["alias"] is not None:
            validate_query_string(member["alias"], f"query_manifest.members[{index}].alias")
        if member["kind"] not in value["member_kinds"]:
            raise EvidenceError(f"query_manifest.members[{index}].kind is not frozen")
        if member["owner_logical_id"] not in seen_types:
            raise EvidenceError(f"query_manifest.members[{index}].owner_logical_id is unknown")
        if member["logical_id"] in seen_members:
            raise EvidenceError(f"duplicate member logical_id in query_manifest: {member['logical_id']}")
        seen_members.add(member["logical_id"])
        member_count_by_owner[member["owner_logical_id"]] += 1
    for ty in types:
        if ty["member_count"] != member_count_by_owner[ty["logical_id"]]:
            raise EvidenceError(f"query_manifest.types member_count mismatch for {ty['logical_id']}")
    lookup = require_object(value["lookup_queries"], "query_manifest.lookup_queries")
    require_exact_keys(lookup, QUERY_LOOKUP_KEYS, "query_manifest.lookup_queries")
    if not isinstance(lookup["type_names"], list):
        raise EvidenceError("query_manifest.lookup_queries.type_names must be an array")
    for index, raw_type in enumerate(lookup["type_names"]):
        query = require_object(raw_type, f"query_manifest.lookup_queries.type_names[{index}]")
        require_exact_keys(query, QUERY_TYPE_LOOKUP_KEYS, f"query_manifest.lookup_queries.type_names[{index}]")
        for key in ("logical_id", "query_name", "query_role"):
            validate_query_string(query[key], f"query_manifest.lookup_queries.type_names[{index}].{key}")
        if query["logical_id"] not in seen_types:
            raise EvidenceError(f"query_manifest.lookup_queries.type_names[{index}] references unknown type")
        if query["query_role"] not in {"primary", "alias"}:
            raise EvidenceError(f"query_manifest.lookup_queries.type_names[{index}].query_role is not frozen")
    expected_type_queries: list[dict[str, Any]] = []
    for ty in types:
        expected_type_queries.append(
            {"logical_id": ty["logical_id"], "query_name": ty["primary"], "query_role": "primary"}
        )
        if ty["alias"] is not None and ty["alias"] != ty["primary"]:
            expected_type_queries.append(
                {"logical_id": ty["logical_id"], "query_name": ty["alias"], "query_role": "alias"}
            )
    if lookup["type_names"] != expected_type_queries:
        raise EvidenceError(
            "query_manifest.lookup_queries.type_names differs from ordered primary/distinct-alias corpus"
        )

    for key in ("properties", "methods"):
        if not isinstance(lookup[key], list):
            raise EvidenceError(f"query_manifest.lookup_queries.{key} must be an array")
        for index, raw_member in enumerate(lookup[key]):
            member = require_object(raw_member, f"query_manifest.lookup_queries.{key}[{index}]")
            require_exact_keys(member, QUERY_MEMBER_LOOKUP_KEYS, f"query_manifest.lookup_queries.{key}[{index}]")
            if member["logical_id"] not in seen_members:
                raise EvidenceError(f"query_manifest.lookup_queries.{key}[{index}] references unknown member")
            if member["owner_logical_id"] not in seen_types:
                raise EvidenceError(f"query_manifest.lookup_queries.{key}[{index}].owner_logical_id is unknown")
            if member["kind"] not in value["member_kinds"]:
                raise EvidenceError(f"query_manifest.lookup_queries.{key}[{index}].kind is not frozen")
            for member_key in ("query_name", "query_role"):
                validate_query_string(member[member_key], f"query_manifest.lookup_queries.{key}[{index}].{member_key}")
            if member["query_role"] not in {"primary", "alias"}:
                raise EvidenceError(f"query_manifest.lookup_queries.{key}[{index}].query_role is not frozen")
        expected_member_queries: list[dict[str, Any]] = []
        expected_kind = "property" if key == "properties" else "method"
        for member in members:
            if member["kind"] != expected_kind:
                continue
            expected_member_queries.append(
                {
                    "logical_id": member["logical_id"],
                    "owner_logical_id": member["owner_logical_id"],
                    "kind": member["kind"],
                    "query_name": member["primary"],
                    "query_role": "primary",
                }
            )
            if member["alias"] is not None and member["alias"] != member["primary"]:
                expected_member_queries.append(
                    {
                        "logical_id": member["logical_id"],
                        "owner_logical_id": member["owner_logical_id"],
                        "kind": member["kind"],
                        "query_name": member["alias"],
                        "query_role": "alias",
                    }
                )
        if lookup[key] != expected_member_queries:
            raise EvidenceError(
                f"query_manifest.lookup_queries.{key} differs from ordered primary/distinct-alias corpus"
            )
    misses = require_object(value["fixed_misses"], "query_manifest.fixed_misses")
    require_exact_keys(misses, QUERY_MISSES_KEYS, "query_manifest.fixed_misses")
    expected_misses = {
        "type_name": "__hbk_s83_av2_missing_type__",
        "member_name": "__hbk_s83_av2_missing_member__",
        "callable_name": "__hbk_s83_av2_missing_callable__",
    }
    if misses != expected_misses:
        raise EvidenceError("query_manifest.fixed_misses differs from frozen S83-AV2 contract")
    anchors = require_object(value["anchors"], "query_manifest.anchors")
    require_exact_keys(anchors, QUERY_ANCHORS_KEYS, "query_manifest.anchors")
    for key in QUERY_ANCHORS_KEYS:
        validate_query_string(anchors[key], f"query_manifest.anchors.{key}")


def validate_orchestration_preflight(value: Any, query_manifest_bytes: int) -> None:
    estimates = require_object(value, "orchestration.preflight_estimates")
    require_exact_keys(estimates, PREFLIGHT_ESTIMATE_KEYS, "orchestration.preflight_estimates")
    if require_integer(estimates["manifest_bytes"], "orchestration.preflight_estimates.manifest_bytes") != query_manifest_bytes:
        raise EvidenceError("orchestration.preflight_estimates.manifest_bytes must equal actual query manifest bytes")
    if require_integer(estimates["manifest_bytes"], "orchestration.preflight_estimates.manifest_bytes") > MAX_MANIFEST_BYTES:
        raise EvidenceError("S83-AV2 manifest exceeds 64 MiB preflight threshold")
    if require_integer(estimates["max_parity_bytes"], "orchestration.preflight_estimates.max_parity_bytes") > MAX_PARITY_BYTES:
        raise EvidenceError("S83-AV2 parity exceeds 512 MiB preflight threshold")
    if require_integer(estimates["raw_run_bytes"], "orchestration.preflight_estimates.raw_run_bytes") > MAX_RAW_BYTES:
        raise EvidenceError("S83-AV2 raw run exceeds 2 GiB preflight threshold")
    expected_cardinality = {
        "operation_context_rows": OPERATION_CONTEXT_ROW_COUNT,
        "parity_rows": PARITY_ROW_COUNT,
        "performance_rows": PERFORMANCE_ROW_COUNT,
    }
    for key, expected_value in expected_cardinality.items():
        if require_integer(estimates[key], f"orchestration.preflight_estimates.{key}") != expected_value:
            raise EvidenceError(f"orchestration.preflight_estimates.{key} differs from frozen S83-AV2 cardinality")


def load_query_manifest(orchestration_path: Path, artifact: dict[str, Any]) -> tuple[Path, str, int]:
    query_path = resolve_path(orchestration_path.parent, artifact["path"]).resolve()
    if not query_path.is_file():
        raise EvidenceError(f"query manifest does not exist: {query_path}")
    raw_bytes = query_path.read_bytes()
    actual = {"bytes": len(raw_bytes), "sha256": sha256_bytes(raw_bytes)}
    if actual != {"bytes": artifact["bytes"], "sha256": artifact["sha256"]}:
        raise EvidenceError("orchestration.query_manifest does not match the actual query manifest artifact")
    if len(raw_bytes) > MAX_MANIFEST_BYTES:
        raise EvidenceError("S83-AV2 manifest exceeds 64 MiB preflight threshold")
    manifest = require_object(json.loads(raw_bytes), "query_manifest")
    reject_forbidden_fields(manifest)
    require_exact_keys(manifest, QUERY_MANIFEST_KEYS, "query_manifest")
    validate_query_manifest_identity(manifest)
    return query_path, actual["sha256"], actual["bytes"]


def load_backends(manifest_path: Path) -> tuple[list[Backend], Path, str, int]:
    raw_bytes = manifest_path.read_bytes()
    orchestration = require_object(json.loads(raw_bytes), "orchestration")
    reject_forbidden_fields(orchestration)
    require_exact_keys(orchestration, ORCHESTRATION_KEYS, "orchestration")
    if orchestration["schema_version"] != ORCHESTRATION_VERSION:
        raise EvidenceError("orchestration.schema_version differs from frozen S83-AV2 contract")
    query_manifest_artifact = validate_artifact(orchestration["query_manifest"], "orchestration.query_manifest")
    query_manifest_path, query_manifest_sha256, query_manifest_bytes = load_query_manifest(manifest_path, query_manifest_artifact)
    validate_orchestration_preflight(orchestration["preflight_estimates"], query_manifest_bytes)
    backends_raw = orchestration["backends"]
    if not isinstance(backends_raw, list) or len(backends_raw) != len(BACKENDS):
        raise EvidenceError("orchestration.backends must contain the exact S83-AV2 backend registry")

    backends: list[Backend] = []
    seen: set[str] = set()
    for index, entry_raw in enumerate(backends_raw):
        entry = require_object(entry_raw, f"orchestration.backends[{index}]")
        require_exact_keys(entry, BACKEND_KEYS, f"orchestration.backends[{index}]")
        backend = entry["backend"]
        role = entry["decision_role"]
        if backend != BACKENDS[index] or not BACKEND_PATTERN.fullmatch(str(backend)):
            raise EvidenceError(f"orchestration.backends[{index}] must be {BACKENDS[index]}")
        if role != DECISION_ROLES[backend]:
            raise EvidenceError(f"{backend}: decision_role must be {DECISION_ROLES[backend]}")
        if backend in seen:
            raise EvidenceError(f"duplicate backend in manifest: {backend}")
        worktree_raw = entry["worktree"]
        if not isinstance(worktree_raw, str) or not worktree_raw:
            raise EvidenceError(f"{backend}: worktree must be a non-empty string")
        command = entry["command"]
        if not isinstance(command, list) or not command or any(not isinstance(part, str) or not part for part in command):
            raise EvidenceError(f"{backend}: command must be a non-empty argv array")
        joined_command = " ".join(command)
        for placeholder in ("{mode}", "{context}", "{query_manifest}"):
            if placeholder not in joined_command:
                raise EvidenceError(f"{backend}: command must contain {placeholder}")
        raw_files = entry["declared_files"]
        if not isinstance(raw_files, list) or not raw_files or any(not isinstance(path, str) or not path for path in raw_files):
            raise EvidenceError(f"{backend}: declared_files must be a non-empty string array")
        worktree = Path(worktree_raw).resolve()
        if git_text(worktree, "status", "--porcelain"):
            raise EvidenceError(f"backend worktree is dirty: {worktree}")
        files = tuple(resolve_path(worktree, path).resolve() for path in raw_files)
        if len(files) != len(set(files)):
            raise EvidenceError(f"{backend}: duplicate declared_files")
        for path in files:
            if not path.is_file():
                raise EvidenceError(f"{backend}: declared file does not exist: {path}")
        executable = resolve_executable(worktree, command[0])
        backends.append(
            Backend(
                backend=backend,
                decision_role=role,
                worktree=worktree,
                command=tuple(command),
                declared_files=files,
                declared_file_artifacts=tuple(artifact_identity(path) for path in files),
                executable=executable,
                executable_artifact=artifact_identity(executable),
                commit=git_text(worktree, "rev-parse", "HEAD"),
                branch=git_text(worktree, "branch", "--show-current"),
            )
        )
        seen.add(backend)
    return backends, query_manifest_path, query_manifest_sha256, query_manifest_bytes


def verify_results_root_empty(results_root: Path) -> None:
    if results_root.exists() and any(results_root.iterdir()):
        raise EvidenceError(f"results root must be empty: {results_root}")
    results_root.mkdir(parents=True, exist_ok=True)


def verify_harness(repo: Path) -> tuple[str, str, dict[str, str]]:
    repo = repo.resolve()
    dirty = git_text(repo, "status", "--porcelain", "--", *HARNESS_PATHS)
    if dirty:
        raise EvidenceError(f"S83-AV2 harness has uncommitted changes:\n{dirty}")
    hashes: dict[str, str] = {}
    for relative in HARNESS_PATHS:
        path = repo / relative
        if not path.is_file():
            raise EvidenceError(f"missing S83-AV2 harness file: {path}")
        hashes[relative] = sha256_file(path)
    return git_text(repo, "rev-parse", "HEAD"), git_text(repo, "branch", "--show-current"), hashes


def host_environment() -> dict[str, Any]:
    return {
        "hostname": socket.gethostname(),
        "kernel": f"{platform.system()} {platform.release()}",
        "architecture": platform.machine(),
        "python": platform.python_version(),
        "rustc": run_text(("rustc", "--version")),
        "cargo": run_text(("cargo", "--version")),
        "logical_cpus": os.cpu_count(),
    }


def machine_state() -> dict[str, Any]:
    load_one, load_five, load_fifteen, scheduler_tasks, last_pid = Path("/proc/loadavg").read_text(encoding="ascii").split()
    runnable, total = scheduler_tasks.split("/", 1)
    return {
        "captured_unix_ns": time.time_ns(),
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
    }


def prepare_files(stance: str, paths: Iterable[Path]) -> dict[str, Any]:
    files = [str(path) for path in paths]
    if stance == "warm":
        for raw_path in files:
            with Path(raw_path).open("rb", buffering=0) as stream:
                while stream.read(8 * 1024 * 1024):
                    pass
        return {"method": "read-declared-files", "declared_files": files}
    if stance != "cold-best-effort":
        raise EvidenceError(f"unknown cache stance: {stance}")
    if not hasattr(os, "posix_fadvise") or not hasattr(os, "POSIX_FADV_DONTNEED"):
        raise EvidenceError("cold-best-effort requires os.posix_fadvise(POSIX_FADV_DONTNEED)")
    subprocess.run(("sync",), check=True)
    for raw_path in files:
        fd = os.open(raw_path, os.O_RDONLY)
        try:
            os.posix_fadvise(fd, 0, 0, os.POSIX_FADV_DONTNEED)
        finally:
            os.close(fd)
    return {"method": "sync+posix_fadvise-dontneed", "declared_files": files, "eviction_verified": False, "claim": "cold-best-effort"}


def normalize_artifact(value: dict[str, Any], worktree: Path) -> tuple[str, int, str]:
    path = resolve_path(worktree, value["path"]).resolve()
    return (str(path), value["bytes"], value["sha256"])


def validate_declared_artifact_binding(report: dict[str, Any], backend: Backend) -> None:
    observed_raw = report["runtime_artifacts"]
    if not isinstance(observed_raw, list) or not observed_raw:
        raise EvidenceError("runtime_artifacts must be a non-empty array")
    observed = []
    for index, artifact in enumerate(observed_raw):
        observed.append(normalize_artifact(validate_artifact(artifact, f"runtime_artifacts[{index}]"), backend.worktree))
    declared = [normalize_artifact(validate_artifact(artifact, "declared_file_artifacts"), backend.worktree) for artifact in backend.declared_file_artifacts]
    if set(observed) != set(declared):
        raise EvidenceError(f"{backend.backend}: declared artifacts do not exactly match runtime_artifacts")


def validate_faults(value: Any, path: str) -> None:
    faults = require_object(value, path)
    require_exact_keys(faults, FAULT_KEYS, path)
    require_integer(faults["minor"], f"{path}.minor")
    require_integer(faults["major"], f"{path}.major")


def validate_timing_phase(value: Any, path: str) -> None:
    phase = require_object(value, path)
    require_exact_keys(phase, TIMING_PHASE_KEYS, path)
    require_integer(phase["elapsed_ns"], f"{path}.elapsed_ns", positive=True)
    require_optional_number(phase["average_ns"], f"{path}.average_ns")
    require_optional_number(phase["ns_per_query"], f"{path}.ns_per_query")
    require_optional_number(phase["ns_per_object"], f"{path}.ns_per_object")
    require_integer(phase["count"], f"{path}.count")
    require_integer(phase["checksum"], f"{path}.checksum")


def validate_allocation_delta(value: Any, path: str) -> None:
    delta = require_object(value, path)
    require_exact_keys(delta, ALLOCATION_DELTA_KEYS, path)
    for key in ALLOCATION_DELTA_KEYS:
        require_integer(delta[key], f"{path}.{key}")


def validate_memory(value: Any, path: str, operation: str) -> None:
    memory = require_object(value, path)
    require_exact_keys(memory, MEMORY_KEYS, path)
    for sample_key in ("before_kib", "live_kib", "after_drop_kib"):
        sample = require_object(memory[sample_key], f"{path}.{sample_key}")
        require_exact_keys(sample, PROCESS_MEMORY_KEYS, f"{path}.{sample_key}")
        for key in PROCESS_MEMORY_KEYS:
            require_integer(sample[key], f"{path}.{sample_key}.{key}")
    for key in MEMORY_KEYS - {"before_kib", "live_kib", "after_drop_kib"}:
        value_int = require_integer(memory[key], f"{path}.{key}")
        if operation not in COMPACT_OPERATIONS and value_int != 0:
            raise EvidenceError(f"{path}.{key} must be zero outside compact_materialization")


def validate_operation_data(value: Any, operation: str) -> dict[str, Any]:
    data = require_object(value, "operation_data")
    tag = OPERATION_TAG[operation]
    expected_keys = {
        "lookup": LOOKUP_DATA_KEYS,
        "iteration": ITERATION_DATA_KEYS,
        "compact_materialization": COMPACT_DATA_KEYS,
        "payload": PAYLOAD_DATA_KEYS,
    }[tag]
    require_exact_keys(data, expected_keys, "operation_data")
    if data["tag"] != tag:
        raise EvidenceError(f"operation_data.tag must be {tag}")
    for key, nested in data.items():
        if key == "tag":
            continue
        require_integer(nested, f"operation_data.{key}")
    if tag == "compact_materialization":
        if data["locator_size"] != 4:
            raise EvidenceError("Av2MemberLocator must be exactly 4 bytes")
        if data["total_len"] > U32_MAX or data["total_capacity"] > U32_MAX:
            raise EvidenceError("compact locator counts must fit u32")
        if data["total_capacity"] < data["total_len"]:
            raise EvidenceError("compact total_capacity must be at least total_len")
        if data["logical_bytes"] != data["total_len"] * 4:
            raise EvidenceError("compact logical_bytes must equal total_len * 4")
        if data["allocated_bytes"] != data["total_capacity"] * 4:
            raise EvidenceError("compact allocated_bytes must equal total_capacity * 4")
    if tag in {"iteration", "compact_materialization"}:
        if data["returned_count"] != data["universal_count"] + data["explicit_count"]:
            raise EvidenceError("returned_count must equal universal_count + explicit_count")
        kind_total = data["property_count"] + data["method_count"] + data["event_count"] + data["enum_value_count"]
        if kind_total != data["returned_count"]:
            raise EvidenceError("member kind counts must equal returned_count")
    return data


def validate_input_identity(value: Any) -> None:
    identity = require_object(value, "input_identity")
    require_exact_keys(identity, INPUT_IDENTITY_KEYS, "input_identity")
    identity_expected = {
        "dataset": DATASET,
        "platform_version": PLATFORM_VERSION,
        "source_locale": SOURCE_LOCALE,
        "provider_schema_version": PROVIDER_SCHEMA_VERSION,
        "extraction_schema_version": EXTRACTION_SCHEMA_VERSION,
    }
    for key, expected_value in identity_expected.items():
        if identity[key] != expected_value:
            raise EvidenceError(f"input_identity.{key} does not identify frozen S83")
    validate_artifact(identity["hbk"], "input_identity.hbk", expected_bytes=HBK_BYTES, expected_sha256=HBK_SHA256)
    validate_artifact(identity["provider"], "input_identity.provider", expected_bytes=PROVIDER_BYTES, expected_sha256=PROVIDER_SHA256)


def validate_manifest_reference(value: Any, manifest_sha256: str, manifest_bytes: int) -> None:
    manifest = require_object(value, "manifest")
    require_exact_keys(manifest, MANIFEST_IDENTITY_KEYS, "manifest")
    if manifest != {"schema_version": MANIFEST_SCHEMA, "sha256": manifest_sha256, "bytes": manifest_bytes}:
        raise EvidenceError("report.manifest does not identify the frozen query manifest")


def validate_parity_report(report: Any, backend: Backend, context: str, manifest_sha256: str, manifest_bytes: int) -> tuple[bytes, dict[str, Any]]:
    report = require_object(report, "parity_report")
    reject_forbidden_fields(report)
    require_exact_keys(report, PARITY_REPORT_KEYS, "parity_report")
    validate_declared_artifact_binding(report, backend)
    expected = {
        "schema_version": PARITY_SCHEMA,
        "workload_version": WORKLOAD_VERSION,
        "mode": "parity",
        "backend": backend.backend,
        "decision_role": backend.decision_role,
        "availability_context": context,
        "module_context_filter_used": False,
        "empty_availability_rule": "universal",
    }
    for key, expected_value in expected.items():
        if report[key] != expected_value:
            raise EvidenceError(f"{backend.backend}/{context}: parity {key} expected {expected_value!r}, got {report[key]!r}")
    validate_input_identity(report["input_identity"])
    validate_manifest_reference(report["manifest"], manifest_sha256, manifest_bytes)
    transcript = report["transcript"]
    if not isinstance(transcript, (dict, list)):
        raise EvidenceError("parity_report.transcript must be a complete JSON object or array")
    transcript_bytes = canonical_json_bytes(transcript)
    if len(transcript_bytes) > MAX_PARITY_BYTES:
        raise EvidenceError("S83-AV2 parity exceeds 512 MiB preflight threshold")
    return transcript_bytes, dict(report)


def validate_report(report: Any, backend: Backend, operation: str, context: str | None, iterations: int, manifest_sha256: str, manifest_bytes: int) -> tuple[bytes | None, dict[str, Any]]:
    report = require_object(report, "report")
    reject_forbidden_fields(report)
    require_exact_keys(report, REPORT_KEYS, "report")
    validate_declared_artifact_binding(report, backend)
    expected = {
        "schema_version": REPORT_SCHEMA,
        "workload_version": WORKLOAD_VERSION,
        "mode": "performance",
        "backend": backend.backend,
        "decision_role": backend.decision_role,
        "operation": operation,
        "availability_context": context,
        "iterations": iterations,
        "module_context_filter_used": False,
        "empty_availability_rule": "universal",
    }
    for key, expected_value in expected.items():
        if report[key] != expected_value:
            raise EvidenceError(f"{backend.backend}/{operation}/{context}: {key} expected {expected_value!r}, got {report[key]!r}")

    validate_input_identity(report["input_identity"])
    validate_manifest_reference(report["manifest"], manifest_sha256, manifest_bytes)

    projection = require_object(report["projection"], "projection")
    require_exact_keys(projection, PROJECTION_KEYS, "projection")
    if projection != PROJECTION_REGISTRY[backend.backend][operation]:
        raise EvidenceError(f"{backend.backend}/{operation}: projection differs from frozen registry")

    if list(report["phase_order"]) != list(PHASE_ORDER):
        raise EvidenceError("phase_order differs from frozen S83-AV2 order")
    timings = require_object(report["timings"], "timings")
    faults = require_object(report["faults"], "faults")
    allocations = require_object(report["allocations"], "allocations")
    require_exact_keys(timings, frozenset(PHASE_ORDER), "timings")
    require_exact_keys(faults, frozenset(PHASE_ORDER), "faults")
    require_exact_keys(allocations, ALLOCATION_KEYS, "allocations")
    if not isinstance(allocations["enabled"], bool):
        raise EvidenceError("allocations.enabled must be boolean")
    for phase in PHASE_ORDER:
        validate_timing_phase(timings[phase], f"timings.{phase}")
        validate_faults(faults[phase], f"faults.{phase}")
        validate_allocation_delta(allocations[phase], f"allocations.{phase}")
    validate_memory(report["memory"], "memory", operation)

    counts = require_object(report["counts"], "counts")
    require_exact_keys(counts, COUNT_KEYS, "counts")
    for key in COUNT_KEYS:
        require_integer(counts[key], f"counts.{key}")
    checksum = require_object(report["checksum"], "checksum")
    require_exact_keys(checksum, frozenset({"value", "algorithm"}), "checksum")
    require_integer(checksum["value"], "checksum.value")
    if checksum["algorithm"] != "rolling-u64":
        raise EvidenceError("checksum.algorithm must be rolling-u64")

    validate_operation_data(report["operation_data"], operation)
    transcript = report.get("canonical_transcript")
    if transcript is not None:
        raise EvidenceError("benchmark report must not contain canonical_transcript")
    if "parity_transcript" in report:
        raise EvidenceError("benchmark report must not contain parity_transcript")
    return None, dict(report)


def execute_report(
    backend: Backend,
    mode: str,
    operation: str | None,
    context: str | None,
    iterations: int | None,
    query_manifest: Path,
    timeout_seconds: int,
    stdout_path: Path,
    stderr_path: Path,
) -> Any:
    command = command_for(backend.command, mode, operation, context, iterations, query_manifest)
    completed = subprocess.run(command, cwd=backend.worktree, text=True, capture_output=True, timeout=timeout_seconds, check=False)
    stdout_path.parent.mkdir(parents=True, exist_ok=True)
    stdout_path.write_text(completed.stdout, encoding="utf-8")
    stderr_path.write_text(completed.stderr, encoding="utf-8")
    if completed.returncode != 0:
        raise EvidenceError(f"{backend.backend}/{operation}/{context}: command failed with {completed.returncode}; see {stderr_path}")
    try:
        return json.loads(completed.stdout)
    except json.JSONDecodeError as error:
        raise EvidenceError(f"{backend.backend}/{operation}/{context}: stdout is not one JSON report; see {stdout_path}") from error


def append_jsonl(path: Path, record: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a", encoding="utf-8") as stream:
        json.dump(record, stream, ensure_ascii=False, separators=(",", ":"))
        stream.write("\n")
        stream.flush()
        os.fsync(stream.fileno())


def parity_key(context: str) -> str:
    return context


def transcript_evidence(transcript_bytes: bytes, baseline_bytes: bytes) -> dict[str, Any]:
    return {
        "sha256": sha256_bytes(transcript_bytes),
        "baseline_sha256": sha256_bytes(baseline_bytes),
        "bytes": len(transcript_bytes),
        "parity_status": "pass" if transcript_bytes == baseline_bytes else "mismatch",
    }


def identity_fields(backend: Backend, harness_commit: str, harness_branch: str, harness_hashes: dict[str, str], manifest_sha256: str, manifest_bytes: int, host: dict[str, Any], samples: int) -> dict[str, Any]:
    return {
        "dataset": DATASET,
        "platform_version": PLATFORM_VERSION,
        "provider_schema_version": PROVIDER_SCHEMA_VERSION,
        "extraction_schema_version": EXTRACTION_SCHEMA_VERSION,
        "hbk_sha256": HBK_SHA256,
        "provider_sha256": PROVIDER_SHA256,
        "backend": backend.backend,
        "decision_role": backend.decision_role,
        "candidate_commit": backend.commit,
        "candidate_branch": backend.branch,
        "worktree": str(backend.worktree),
        "executable_artifact": dict(backend.executable_artifact),
        "harness_commit": harness_commit,
        "harness_branch": harness_branch,
        "harness_file_sha256": harness_hashes,
        "manifest_sha256": manifest_sha256,
        "manifest_bytes": manifest_bytes,
        "host": host,
        "orchestration_version": ORCHESTRATION_VERSION,
        "backend_registry": list(BACKENDS),
        "operation_registry": list(OPERATIONS),
        "availability_context_registry": list(AVAILABILITY_CONTEXTS),
        "cache_stance_registry": list(CACHE_STANCES),
        "planned_samples_per_row": samples,
        "declared_file_artifacts": [dict(artifact) for artifact in backend.declared_file_artifacts],
    }


def parity_plan(backends: Sequence[Backend]) -> Iterator[tuple[Backend, str]]:
    h0 = next(backend for backend in backends if backend.backend == "S83-H0")
    for context in AVAILABILITY_CONTEXTS:
        yield h0, context
    for backend in backends:
        if backend.backend == "S83-H0":
            continue
        for context in AVAILABILITY_CONTEXTS:
            yield backend, context


def smoke_plan(backends: Sequence[Backend]) -> Iterator[Backend]:
    for backend in backends:
        yield backend


def add_parity_bytes(totals: dict[str, int], backend: str, size: int) -> None:
    totals[backend] += size
    if totals[backend] > MAX_PARITY_BYTES:
        raise EvidenceError(
            f"{backend}: canonical parity for all availability contexts exceeds 512 MiB"
        )


def measurement_plan(backends: Sequence[Backend], samples: int) -> Iterator[tuple[Backend, str, str | None, str, int]]:
    # One child at a time, but backend order is round-robined within each
    # sample/context/stance/operation cell to reduce temporal bias.
    context_slots: tuple[str | None, ...] = (None, *AVAILABILITY_CONTEXTS)
    for sample in range(1, samples + 1):
        for context in context_slots:
            for stance in CACHE_STANCES:
                for operation in OPERATIONS:
                    if context not in operation_contexts(operation):
                        continue
                    for backend in backends:
                        yield backend, operation, context, stance, sample


def parity_phase(backends: Sequence[Backend], results_root: Path, query_manifest: Path, timeout_seconds: int, base_identity: dict[str, dict[str, Any]], manifest_sha256: str, manifest_bytes: int) -> dict[str, bytes]:
    parity_jsonl = results_root / "parity" / "parity.jsonl"
    if parity_jsonl.exists():
        raise EvidenceError(f"refusing to append to existing parity evidence: {parity_jsonl}")
    baseline: dict[str, bytes] = {}
    parity_bytes_by_backend: dict[str, int] = {backend.backend: 0 for backend in backends}
    passed_rows = 0
    for backend, context in parity_plan(backends):
        slug = f"{backend.backend}-{context}"
        stdout_path = results_root / "logs" / "parity" / f"{slug}.stdout.json"
        stderr_path = results_root / "logs" / "parity" / f"{slug}.stderr.log"
        report = execute_report(backend, "parity", None, context, None, query_manifest, timeout_seconds, stdout_path, stderr_path)
        transcript, stripped = validate_parity_report(report, backend, context, manifest_sha256, manifest_bytes)
        add_parity_bytes(parity_bytes_by_backend, backend.backend, len(transcript))
        key = parity_key(context)
        if backend.backend == "S83-H0":
            baseline[key] = transcript
            transcript_path = results_root / "parity" / "h0" / f"{context}.json"
            transcript_path.parent.mkdir(parents=True, exist_ok=True)
            transcript_path.write_bytes(transcript + b"\n")
        if key not in baseline:
            raise EvidenceError("internal error: H0 byte parity must run before other backends")
        evidence = transcript_evidence(transcript, baseline[key])
        stdout_path.write_bytes(canonical_json_bytes({"parity": stripped, "transcript": evidence}) + b"\n")
        record = {
            "schema": PARITY_SCHEMA,
            **base_identity[backend.backend],
            "availability_context": context,
            "module_context_filter_used": False,
            "empty_availability_rule": "universal",
            "command_template": list(backend.command),
            "command": command_for(backend.command, "parity", None, context, None, query_manifest),
            "stdout_log": str(stdout_path),
            "stderr_log": str(stderr_path),
            "transcript": evidence,
        }
        append_jsonl(parity_jsonl, record)
        if evidence["parity_status"] != "pass":
            raise EvidenceError(f"{backend.backend}/{context}: transcript differs byte-for-byte from S83-H0")
        passed_rows += 1
    if passed_rows != PARITY_ROW_COUNT:
        raise EvidenceError(f"parity did not complete the frozen 81-row matrix: {passed_rows}")
    return baseline


def smoke_phase(backends: Sequence[Backend], results_root: Path, baseline: dict[str, bytes], query_manifest: Path, timeout_seconds: int, base_identity: dict[str, dict[str, Any]], manifest_sha256: str, manifest_bytes: int) -> set[str]:
    if len(baseline) != len(AVAILABILITY_CONTEXTS):
        raise EvidenceError("preflight smoke requires all 81 parity rows to pass first")
    smoke_jsonl = results_root / "preflight" / "smoke.jsonl"
    if smoke_jsonl.exists():
        raise EvidenceError(f"refusing to append to existing preflight smoke evidence: {smoke_jsonl}")
    passed: set[str] = set()
    max_report_bytes = 0
    for backend in smoke_plan(backends):
        slug = f"{backend.backend}-{SMOKE_OPERATION}-{SMOKE_CONTEXT}"
        stdout_path = results_root / "logs" / "preflight-smoke" / f"{slug}.stdout.json"
        stderr_path = results_root / "logs" / "preflight-smoke" / f"{slug}.stderr.log"
        report = execute_report(
            backend,
            "performance",
            SMOKE_OPERATION,
            SMOKE_CONTEXT,
            SMOKE_ITERATIONS,
            query_manifest,
            timeout_seconds,
            stdout_path,
            stderr_path,
        )
        transcript, stripped = validate_report(report, backend, SMOKE_OPERATION, SMOKE_CONTEXT, SMOKE_ITERATIONS, manifest_sha256, manifest_bytes)
        if transcript is not None:
            raise EvidenceError("preflight smoke performance report must not contain parity_transcript")
        smoke_bytes = len(canonical_json_bytes(stripped))
        max_report_bytes = max(max_report_bytes, smoke_bytes)
        if max_report_bytes * PERFORMANCE_ROW_COUNT > MAX_RAW_BYTES:
            raise EvidenceError("S83-AV2 raw run exceeds 2 GiB preflight threshold based on smoke report size")
        record = {
            "schema": PREFLIGHT_SMOKE_SCHEMA,
            **base_identity[backend.backend],
            "operation": SMOKE_OPERATION,
            "availability_context": SMOKE_CONTEXT,
            "iterations": SMOKE_ITERATIONS,
            "status": "ok",
            "command_template": list(backend.command),
            "command": command_for(backend.command, "performance", SMOKE_OPERATION, SMOKE_CONTEXT, SMOKE_ITERATIONS, query_manifest),
            "stdout_log": str(stdout_path),
            "stderr_log": str(stderr_path),
            "stderr_sha256": sha256_file(stderr_path),
            "report_bytes": smoke_bytes,
            "max_observed_report_bytes": max_report_bytes,
            "measurement": stripped,
        }
        append_jsonl(smoke_jsonl, record)
        passed.add(backend.backend)
    if passed != set(BACKENDS):
        raise EvidenceError(f"preflight smoke did not complete the frozen 9-row backend matrix: {len(passed)}")
    return passed


def measurement_phase(backends: Sequence[Backend], results_root: Path, baseline: dict[str, bytes], smoke_passed: set[str], samples: int, lookup_iterations: int, payload_iterations: int, enumeration_iterations: int, query_manifest: Path, timeout_seconds: int, base_identity: dict[str, dict[str, Any]], manifest_sha256: str, manifest_bytes: int) -> None:
    if len(baseline) != len(AVAILABILITY_CONTEXTS):
        raise EvidenceError("performance requires all 81 parity rows to pass before measurement")
    if smoke_passed != set(BACKENDS):
        raise EvidenceError("performance requires all 9 preflight smoke rows to pass before measurement")
    raw_path = results_root / "raw" / "measurements.jsonl"
    if raw_path.exists():
        raise EvidenceError(f"refusing to append to existing raw evidence: {raw_path}")
    completed_rows = 0
    for backend, operation, context, stance, sample in measurement_plan(backends, samples):
        iterations = iterations_for(operation, lookup_iterations, payload_iterations, enumeration_iterations)
        preparation = prepare_files(stance, backend.declared_files)
        before = machine_state()
        slug = f"{sample:02d}-{context or 'none'}-{stance}-{operation}-{backend.backend}"
        stdout_path = results_root / "logs" / "measurements" / f"{slug}.stdout.json"
        stderr_path = results_root / "logs" / "measurements" / f"{slug}.stderr.log"
        report = execute_report(backend, "performance", operation, context, iterations, query_manifest, timeout_seconds, stdout_path, stderr_path)
        after = machine_state()
        transcript, stripped = validate_report(report, backend, operation, context, iterations, manifest_sha256, manifest_bytes)
        if transcript is not None:
            raise EvidenceError("raw performance report must not contain parity_transcript")
        record = {
            "schema": RAW_SCHEMA,
            **base_identity[backend.backend],
            "operation": operation,
            "availability_context": context,
            "cache_stance": stance,
            "sample": sample,
            "iterations": iterations,
            "status": "ok",
            "command_template": list(backend.command),
            "command": command_for(backend.command, "performance", operation, context, iterations, query_manifest),
            "declared_files": [str(path) for path in backend.declared_files],
            "preparation": preparation,
            "machine_state_before": before,
            "machine_state_after": after,
            "stdout_log": str(stdout_path),
            "stderr_log": str(stderr_path),
            "stderr_sha256": sha256_file(stderr_path),
            "h0_parity_sha256": sha256_bytes(baseline[parity_key(context or AVAILABILITY_CONTEXTS[0])]),
            "measurement": stripped,
        }
        append_jsonl(raw_path, record)
        completed_rows += 1
    if completed_rows != PERFORMANCE_ROW_COUNT:
        raise EvidenceError(
            f"performance did not complete the frozen {PERFORMANCE_ROW_COUNT}-row matrix: {completed_rows}"
        )


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        validate_frozen_run_parameters(args)
        repo = args.repo.resolve()
        results_root = args.results_root.resolve()
        verify_results_root_empty(results_root)
        backends, query_manifest, manifest_sha256, manifest_bytes = load_backends(args.manifest.resolve())
        harness_commit, harness_branch, harness_hashes = verify_harness(repo)
        host = host_environment()
        base_identity = {
            backend.backend: identity_fields(
                backend,
                harness_commit,
                harness_branch,
                harness_hashes,
                manifest_sha256,
                manifest_bytes,
                host,
                args.samples,
            )
            for backend in backends
        }
        baseline = parity_phase(backends, results_root, query_manifest, args.timeout_seconds, base_identity, manifest_sha256, manifest_bytes)
        smoke_passed = smoke_phase(backends, results_root, baseline, query_manifest, args.timeout_seconds, base_identity, manifest_sha256, manifest_bytes)
        measurement_phase(
            backends,
            results_root,
            baseline,
            smoke_passed,
            args.samples,
            args.lookup_iterations,
            args.payload_iterations,
            args.enumeration_iterations,
            query_manifest,
            args.timeout_seconds,
            base_identity,
            manifest_sha256,
            manifest_bytes,
        )
        print(results_root / "raw" / "measurements.jsonl")
        return 0
    except (EvidenceError, OSError, subprocess.SubprocessError, json.JSONDecodeError) as error:
        print(f"S83-AV2 evidence failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
