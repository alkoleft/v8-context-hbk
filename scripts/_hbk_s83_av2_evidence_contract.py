#!/usr/bin/env python3
"""Frozen S83-AV2 evidence schema and validator used by the result summarizer.

This module is not a benchmark producer or runtime dependency.
"""

from __future__ import annotations

import json
import re
from dataclasses import dataclass
from pathlib import Path
from typing import Any

MANIFEST_SCHEMA = 'hbk-s83-av2-query-manifest/v1'

REPORT_SCHEMA = 'hbk-s83-av2-benchmark/v1'

RAW_SCHEMA = 'hbk-s83-av2-raw/v1'

SUMMARY_SCHEMA = 'hbk-s83-av2-summary/v1'

WORKLOAD_VERSION = 's83-av2-context-member-access/v1'

ORCHESTRATION_VERSION = 'hbk-s83-av2-orchestration/v1'

DATASET = 'shcntx_ru-8.3.27.1859-schema16-extraction11'

PLATFORM_VERSION = '8.3.27.1859'

SOURCE_LOCALE = 'ru'

PROVIDER_SCHEMA_VERSION = 16

EXTRACTION_SCHEMA_VERSION = 11

HBK_BYTES = 40744845

HBK_SHA256 = '5bdf0b3ed89932572c012faddc4d05ebfa2986595cf2849b6eb6e5e65a9a4d48'

PROVIDER_BYTES = 204288000

PROVIDER_SHA256 = '55c2e09971712a13a49cbcf5889f203d7a9dfcec22aa0d333247ae722f6f0fab'

BACKENDS = ('S83-H0', 'S83-C0', 'S83-F0', 'S83-A0', 'S83-L1', 'S83-I1', 'S83-D1', 'S83-P1', 'S83-R1')

DECISION_ROLES = {'S83-H0': 'baseline', 'S83-C0': 'control', 'S83-F0': 'candidate', 'S83-A0': 'candidate', 'S83-L1': 'candidate', 'S83-I1': 'candidate', 'S83-D1': 'candidate', 'S83-P1': 'candidate', 'S83-R1': 'candidate'}

AVAILABILITY_CONTEXTS = ('thin_client', 'web_client', 'mobile_client', 'server', 'thick_client', 'external_connection', 'mobile_application_client', 'mobile_application_server', 'mobile_standalone_server')

CACHE_STANCES = ('warm', 'cold-best-effort')

OPERATIONS = ('type_by_name', 'property_by_owner_name_kind', 'method_by_owner_name_kind', 'callable_by_owner_name', 'members_by_owner_availability_borrowed', 'members_by_owner_availability_collect', 'type_payload', 'method_payload', 'property_payload', 'filtered_members_payload')

LOOKUP_OPERATIONS = frozenset({'type_by_name', 'property_by_owner_name_kind', 'method_by_owner_name_kind', 'callable_by_owner_name'})

COMPACT_OPERATIONS = frozenset({'members_by_owner_availability_collect'})

PAYLOAD_OPERATIONS = frozenset({'type_payload', 'method_payload', 'property_payload', 'filtered_members_payload'})

CONTEXTUAL_OPERATIONS = frozenset({'members_by_owner_availability_borrowed', 'members_by_owner_availability_collect', 'filtered_members_payload'})

OPERATION_TAG = {**{operation: 'lookup' for operation in LOOKUP_OPERATIONS}, 'members_by_owner_availability_borrowed': 'iteration', 'members_by_owner_availability_collect': 'compact_materialization', **{operation: 'payload' for operation in PAYLOAD_OPERATIONS}}

PHASE_ORDER = ('entry_to_ready', 'anchor_resolution', 'first_operation', 'warmup', 'steady_workload', 'memory_sample')

DEFAULT_SAMPLES = 9

OPERATION_CONTEXT_ROW_COUNT = sum(
    len(AVAILABILITY_CONTEXTS) if operation in CONTEXTUAL_OPERATIONS else 1
    for operation in OPERATIONS
)

PARITY_ROW_COUNT = len(BACKENDS) * len(AVAILABILITY_CONTEXTS)

PERFORMANCE_ROW_COUNT = (
    OPERATION_CONTEXT_ROW_COUNT * len(BACKENDS) * len(CACHE_STANCES) * DEFAULT_SAMPLES
)

U32_MAX = 2 ** 32 - 1

FORBIDDEN_MODULE_CONTEXT_KEY = re.compile('module_?context_?kind', re.IGNORECASE)

FORBIDDEN_RANKING_KEY = re.compile('(^|_)(rank|score|winner|recommendation)(_|$)', re.IGNORECASE)

FORBIDDEN_COMPACT_PAYLOAD_KEY = re.compile('(retained_.*(payload|string|dto)|compact_.*(payload|string|dto)|resolver_?dto)', re.IGNORECASE)

ARTIFACT_KEYS = frozenset({'path', 'bytes', 'sha256'})

REPORT_KEYS = frozenset({'schema_version', 'workload_version', 'mode', 'backend', 'decision_role', 'operation', 'availability_context', 'iterations', 'module_context_filter_used', 'empty_availability_rule', 'input_identity', 'manifest', 'runtime_artifacts', 'projection', 'phase_order', 'timings', 'faults', 'allocations', 'memory', 'counts', 'checksum', 'operation_data'})

INPUT_IDENTITY_KEYS = frozenset({'dataset', 'platform_version', 'source_locale', 'provider_schema_version', 'extraction_schema_version', 'hbk', 'provider'})

MANIFEST_IDENTITY_KEYS = frozenset({'schema_version', 'sha256', 'bytes'})

PROJECTION_KEYS = frozenset({'source', 'compact'})

FAULT_KEYS = frozenset({'minor', 'major'})

TIMING_PHASE_KEYS = frozenset({'elapsed_ns', 'average_ns', 'ns_per_query', 'ns_per_object', 'count', 'checksum'})

ALLOCATION_DELTA_KEYS = frozenset({'allocation_calls', 'reallocation_calls', 'deallocation_calls', 'allocated_bytes', 'deallocated_bytes', 'live_bytes_before', 'live_bytes_after', 'peak_live_bytes_before', 'peak_live_bytes_after', 'peak_live_bytes_growth'})

ALLOCATION_KEYS = frozenset({'enabled', *PHASE_ORDER})

MEMORY_KEYS = frozenset({'before_kib', 'live_kib', 'after_drop_kib', 'container_overhead_bytes', 'logical_bytes', 'capacity_bytes', 'live_delta_bytes', 'peak_live_delta_bytes', 'post_drop_delta_bytes'})

PROCESS_MEMORY_KEYS = frozenset({'rss_kib', 'pss_kib', 'private_kib', 'anonymous_kib', 'file_backed_kib'})

COUNT_KEYS = frozenset({'query_count', 'candidate_count', 'object_count', 'checksum_count', 'property_count', 'method_count', 'event_count', 'enum_value_count'})

LOOKUP_DATA_KEYS = frozenset({'tag', 'query_count', 'candidate_count', 'miss_count'})

ITERATION_DATA_KEYS = frozenset({'tag', 'owner_count', 'scanned_count', 'returned_count', 'universal_count', 'explicit_count', 'excluded_count', 'property_count', 'method_count', 'event_count', 'enum_value_count'})

COMPACT_DATA_KEYS = frozenset({*ITERATION_DATA_KEYS, 'locator_size', 'total_len', 'total_capacity', 'logical_bytes', 'allocated_bytes'})

PAYLOAD_DATA_KEYS = frozenset({'tag', 'input_count', 'object_count', 'string_bytes_touched', 'canonical_payload_bytes_touched'})

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

def require_object(value: Any, path: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise EvidenceError(f'{path} must be an object')
    return value

def require_exact_keys(value: dict[str, Any], expected: frozenset[str], path: str) -> None:
    actual = set(value)
    if actual != expected:
        raise EvidenceError(f'{path} schema keys differ: missing={sorted(expected - actual)}, unexpected={sorted(actual - expected)}')

def require_integer(value: Any, path: str, *, positive: bool=False) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        raise EvidenceError(f'{path} must be an integer')
    if positive and value <= 0:
        raise EvidenceError(f'{path} must be greater than zero')
    if not positive and value < 0:
        raise EvidenceError(f'{path} must be non-negative')
    return value

def require_optional_number(value: Any, path: str) -> None:
    if value is None:
        return
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise EvidenceError(f'{path} must be numeric or null')
    if value < 0:
        raise EvidenceError(f'{path} must be non-negative')

def reject_forbidden_fields(value: Any, path: str='') -> None:
    if isinstance(value, dict):
        for (key, nested) in value.items():
            key_text = str(key)
            nested_path = f'{path}.{key_text}' if path else key_text
            if key_text != 'module_context_filter_used' and FORBIDDEN_MODULE_CONTEXT_KEY.search(key_text):
                raise EvidenceError(f'ModuleContextKind is forbidden at {nested_path}')
            if FORBIDDEN_RANKING_KEY.search(key_text):
                raise EvidenceError(f'ranking/selection field is forbidden at {nested_path}')
            if FORBIDDEN_COMPACT_PAYLOAD_KEY.search(key_text):
                raise EvidenceError(f'retained compact payload/string/DTO field is forbidden at {nested_path}')
            reject_forbidden_fields(nested, nested_path)
    elif isinstance(value, list):
        for (index, nested) in enumerate(value):
            reject_forbidden_fields(nested, f'{path}[{index}]')

def validate_artifact(value: Any, path: str, *, expected_bytes: int | None=None, expected_sha256: str | None=None) -> dict[str, Any]:
    artifact = require_object(value, path)
    require_exact_keys(artifact, ARTIFACT_KEYS, path)
    raw_path = artifact['path']
    if not isinstance(raw_path, str) or not raw_path:
        raise EvidenceError(f'{path}.path must be a non-empty string')
    size = require_integer(artifact['bytes'], f'{path}.bytes', positive=True)
    digest = artifact['sha256']
    if not isinstance(digest, str) or not re.fullmatch('[0-9a-f]{64}', digest):
        raise EvidenceError(f'{path}.sha256 must be a lowercase SHA-256')
    if expected_bytes is not None and size != expected_bytes:
        raise EvidenceError(f'{path}.bytes does not identify frozen S83')
    if expected_sha256 is not None and digest != expected_sha256:
        raise EvidenceError(f'{path}.sha256 does not identify frozen S83')
    return artifact

def resolve_path(worktree: Path, raw: str) -> Path:
    path = Path(raw)
    return path if path.is_absolute() else worktree / path

def source_projection_for(backend: str, operation: str) -> str:
    if operation in PAYLOAD_OPERATIONS:
        if backend in {'S83-H0', 'S83-C0'}:
            return 'owned-reference'
        if backend == 'S83-A0':
            return 'archived-view'
        if backend == 'S83-R1':
            return 'borrowed-range'
        return 'decoded-value'
    if backend in {'S83-H0', 'S83-C0'}:
        return 'owned-id-slice'
    if backend == 'S83-A0':
        return 'archived-id-range'
    return 'mapped-id-range'

def compact_projection_for(operation: str) -> str | None:
    return 'av2-member-locator-u32' if operation in COMPACT_OPERATIONS else None

PROJECTION_REGISTRY = {backend: {operation: {'source': source_projection_for(backend, operation), 'compact': compact_projection_for(operation)} for operation in OPERATIONS} for backend in BACKENDS}

def operation_contexts(operation: str) -> tuple[str | None, ...]:
    return AVAILABILITY_CONTEXTS if operation in CONTEXTUAL_OPERATIONS else (None,)

def normalize_artifact(value: dict[str, Any], worktree: Path) -> tuple[str, int, str]:
    path = resolve_path(worktree, value['path']).resolve()
    return (str(path), value['bytes'], value['sha256'])

def validate_declared_artifact_binding(report: dict[str, Any], backend: Backend) -> None:
    observed_raw = report['runtime_artifacts']
    if not isinstance(observed_raw, list) or not observed_raw:
        raise EvidenceError('runtime_artifacts must be a non-empty array')
    observed = []
    for (index, artifact) in enumerate(observed_raw):
        observed.append(normalize_artifact(validate_artifact(artifact, f'runtime_artifacts[{index}]'), backend.worktree))
    declared = [normalize_artifact(validate_artifact(artifact, 'declared_file_artifacts'), backend.worktree) for artifact in backend.declared_file_artifacts]
    if set(observed) != set(declared):
        raise EvidenceError(f'{backend.backend}: declared artifacts do not exactly match runtime_artifacts')

def validate_faults(value: Any, path: str) -> None:
    faults = require_object(value, path)
    require_exact_keys(faults, FAULT_KEYS, path)
    require_integer(faults['minor'], f'{path}.minor')
    require_integer(faults['major'], f'{path}.major')

def validate_timing_phase(value: Any, path: str) -> None:
    phase = require_object(value, path)
    require_exact_keys(phase, TIMING_PHASE_KEYS, path)
    require_integer(phase['elapsed_ns'], f'{path}.elapsed_ns', positive=True)
    require_optional_number(phase['average_ns'], f'{path}.average_ns')
    require_optional_number(phase['ns_per_query'], f'{path}.ns_per_query')
    require_optional_number(phase['ns_per_object'], f'{path}.ns_per_object')
    require_integer(phase['count'], f'{path}.count')
    require_integer(phase['checksum'], f'{path}.checksum')

def validate_allocation_delta(value: Any, path: str) -> None:
    delta = require_object(value, path)
    require_exact_keys(delta, ALLOCATION_DELTA_KEYS, path)
    for key in ALLOCATION_DELTA_KEYS:
        require_integer(delta[key], f'{path}.{key}')

def validate_memory(value: Any, path: str, operation: str) -> None:
    memory = require_object(value, path)
    require_exact_keys(memory, MEMORY_KEYS, path)
    for sample_key in ('before_kib', 'live_kib', 'after_drop_kib'):
        sample = require_object(memory[sample_key], f'{path}.{sample_key}')
        require_exact_keys(sample, PROCESS_MEMORY_KEYS, f'{path}.{sample_key}')
        for key in PROCESS_MEMORY_KEYS:
            require_integer(sample[key], f'{path}.{sample_key}.{key}')
    for key in MEMORY_KEYS - {'before_kib', 'live_kib', 'after_drop_kib'}:
        value_int = require_integer(memory[key], f'{path}.{key}')
        if operation not in COMPACT_OPERATIONS and value_int != 0:
            raise EvidenceError(f'{path}.{key} must be zero outside compact_materialization')

def validate_operation_data(value: Any, operation: str) -> dict[str, Any]:
    data = require_object(value, 'operation_data')
    tag = OPERATION_TAG[operation]
    expected_keys = {'lookup': LOOKUP_DATA_KEYS, 'iteration': ITERATION_DATA_KEYS, 'compact_materialization': COMPACT_DATA_KEYS, 'payload': PAYLOAD_DATA_KEYS}[tag]
    require_exact_keys(data, expected_keys, 'operation_data')
    if data['tag'] != tag:
        raise EvidenceError(f'operation_data.tag must be {tag}')
    for (key, nested) in data.items():
        if key == 'tag':
            continue
        require_integer(nested, f'operation_data.{key}')
    if tag == 'compact_materialization':
        if data['locator_size'] != 4:
            raise EvidenceError('Av2MemberLocator must be exactly 4 bytes')
        if data['total_len'] > U32_MAX or data['total_capacity'] > U32_MAX:
            raise EvidenceError('compact locator counts must fit u32')
        if data['total_capacity'] < data['total_len']:
            raise EvidenceError('compact total_capacity must be at least total_len')
        if data['logical_bytes'] != data['total_len'] * 4:
            raise EvidenceError('compact logical_bytes must equal total_len * 4')
        if data['allocated_bytes'] != data['total_capacity'] * 4:
            raise EvidenceError('compact allocated_bytes must equal total_capacity * 4')
    if tag in {'iteration', 'compact_materialization'}:
        if data['returned_count'] != data['universal_count'] + data['explicit_count']:
            raise EvidenceError('returned_count must equal universal_count + explicit_count')
        kind_total = data['property_count'] + data['method_count'] + data['event_count'] + data['enum_value_count']
        if kind_total != data['returned_count']:
            raise EvidenceError('member kind counts must equal returned_count')
    return data

def validate_input_identity(value: Any) -> None:
    identity = require_object(value, 'input_identity')
    require_exact_keys(identity, INPUT_IDENTITY_KEYS, 'input_identity')
    identity_expected = {'dataset': DATASET, 'platform_version': PLATFORM_VERSION, 'source_locale': SOURCE_LOCALE, 'provider_schema_version': PROVIDER_SCHEMA_VERSION, 'extraction_schema_version': EXTRACTION_SCHEMA_VERSION}
    for (key, expected_value) in identity_expected.items():
        if identity[key] != expected_value:
            raise EvidenceError(f'input_identity.{key} does not identify frozen S83')
    validate_artifact(identity['hbk'], 'input_identity.hbk', expected_bytes=HBK_BYTES, expected_sha256=HBK_SHA256)
    validate_artifact(identity['provider'], 'input_identity.provider', expected_bytes=PROVIDER_BYTES, expected_sha256=PROVIDER_SHA256)

def validate_manifest_reference(value: Any, manifest_sha256: str, manifest_bytes: int) -> None:
    manifest = require_object(value, 'manifest')
    require_exact_keys(manifest, MANIFEST_IDENTITY_KEYS, 'manifest')
    if manifest != {'schema_version': MANIFEST_SCHEMA, 'sha256': manifest_sha256, 'bytes': manifest_bytes}:
        raise EvidenceError('report.manifest does not identify the frozen query manifest')

def validate_report(report: Any, backend: Backend, operation: str, context: str | None, iterations: int, manifest_sha256: str, manifest_bytes: int) -> tuple[bytes | None, dict[str, Any]]:
    report = require_object(report, 'report')
    reject_forbidden_fields(report)
    require_exact_keys(report, REPORT_KEYS, 'report')
    validate_declared_artifact_binding(report, backend)
    expected = {'schema_version': REPORT_SCHEMA, 'workload_version': WORKLOAD_VERSION, 'mode': 'performance', 'backend': backend.backend, 'decision_role': backend.decision_role, 'operation': operation, 'availability_context': context, 'iterations': iterations, 'module_context_filter_used': False, 'empty_availability_rule': 'universal'}
    for (key, expected_value) in expected.items():
        if report[key] != expected_value:
            raise EvidenceError(f'{backend.backend}/{operation}/{context}: {key} expected {expected_value!r}, got {report[key]!r}')
    validate_input_identity(report['input_identity'])
    validate_manifest_reference(report['manifest'], manifest_sha256, manifest_bytes)
    projection = require_object(report['projection'], 'projection')
    require_exact_keys(projection, PROJECTION_KEYS, 'projection')
    if projection != PROJECTION_REGISTRY[backend.backend][operation]:
        raise EvidenceError(f'{backend.backend}/{operation}: projection differs from frozen registry')
    if list(report['phase_order']) != list(PHASE_ORDER):
        raise EvidenceError('phase_order differs from frozen S83-AV2 order')
    timings = require_object(report['timings'], 'timings')
    faults = require_object(report['faults'], 'faults')
    allocations = require_object(report['allocations'], 'allocations')
    require_exact_keys(timings, frozenset(PHASE_ORDER), 'timings')
    require_exact_keys(faults, frozenset(PHASE_ORDER), 'faults')
    require_exact_keys(allocations, ALLOCATION_KEYS, 'allocations')
    if not isinstance(allocations['enabled'], bool):
        raise EvidenceError('allocations.enabled must be boolean')
    for phase in PHASE_ORDER:
        validate_timing_phase(timings[phase], f'timings.{phase}')
        validate_faults(faults[phase], f'faults.{phase}')
        validate_allocation_delta(allocations[phase], f'allocations.{phase}')
    validate_memory(report['memory'], 'memory', operation)
    counts = require_object(report['counts'], 'counts')
    require_exact_keys(counts, COUNT_KEYS, 'counts')
    for key in COUNT_KEYS:
        require_integer(counts[key], f'counts.{key}')
    checksum = require_object(report['checksum'], 'checksum')
    require_exact_keys(checksum, frozenset({'value', 'algorithm'}), 'checksum')
    require_integer(checksum['value'], 'checksum.value')
    if checksum['algorithm'] != 'rolling-u64':
        raise EvidenceError('checksum.algorithm must be rolling-u64')
    validate_operation_data(report['operation_data'], operation)
    transcript = report.get('canonical_transcript')
    if transcript is not None:
        raise EvidenceError('benchmark report must not contain canonical_transcript')
    if 'parity_transcript' in report:
        raise EvidenceError('benchmark report must not contain parity_transcript')
    return (None, dict(report))
