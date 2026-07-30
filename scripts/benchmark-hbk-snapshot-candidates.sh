#!/usr/bin/env bash
set -euo pipefail

readonly SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
readonly REPO_ROOT="$(cd -- "${SCRIPT_DIR}/.." && pwd)"
readonly RESULTS_ROOT="${REPO_ROOT}/target/hbk-zero-copy-experiment"
readonly RAW_RESULTS="${RESULTS_ROOT}/results/raw-v1.jsonl"
readonly LOG_DIR="${RESULTS_ROOT}/logs"
readonly RUN_DIR="${RESULTS_ROOT}/runs"
readonly PREPARED_CACHE="${RESULTS_ROOT}/prepared/current-cache.bin"
readonly SQLITE_PATH="${REPO_ROOT}/target/snapshot-materialization/shcntx_ru.schema16.release.sqlite"
readonly HBK_PATH="/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk"
readonly SQLITE_SHA256="cc9b2b8aaf31f64c880b92cc3a02fd3166541f10f8d209faf8c7a7c22cac0d55"
readonly HBK_SHA256="b8bc0d3a1ee8d00e2f113a800339731304428cc35ae395e5094a8b022773f8ed"
readonly DATASET_ID="shcntx_ru-8.5.1.1150-schema16-extraction11"
readonly PLATFORM_VERSION="8.5.1.1150"
readonly PROVIDER_SCHEMA_VERSION=16
readonly EXTRACTION_SCHEMA_VERSION=11
readonly EXAMPLE_BIN="${REPO_ROOT}/target/release/examples/measure_hbk_snapshot_scenario"
readonly ORACLE_BIN="${REPO_ROOT}/target/release/examples/dump_hbk_snapshot_oracle"
readonly ALLOCATION_TARGET="${RESULTS_ROOT}/allocation-target"
readonly ALLOCATION_EXAMPLE_BIN="${ALLOCATION_TARGET}/release/examples/measure_hbk_snapshot_scenario"
readonly DEFAULT_ITERATIONS=20000
readonly DEFAULT_RUNS=9
readonly DEFAULT_ALLOCATION_RUNS=3
readonly WARMUP_RUNS=2

usage() {
    {
        echo "usage:"
        echo "  $0 verify"
        echo "  $0 build"
        echo "  $0 prepare-cache [iterations]"
        echo "  $0 parity-baseline"
        echo "  $0 record-parity <backend> <content.jsonl> <lookups.jsonl>"
        echo "  $0 allocation-baseline <sql-owned|cache-owned> [runs] [iterations]"
        echo "  $0 baseline <warm|cold-best-effort> [runs] [iterations]"
        echo "  $0 run-command <backend> <warm|cold-best-effort> <sample> <evict-paths-or--> -- <command> [args...]"
        echo "  $0 multi-reader-baseline <sql-owned|cache-owned> [runs] [iterations]"
        echo "  $0 paths"
    } >&2
}

require_tool() {
    if ! command -v "$1" >/dev/null 2>&1; then
        echo "required tool is unavailable: $1" >&2
        exit 2
    fi
}

verify_sha256() {
    local path="$1"
    local expected="$2"
    local actual
    actual="$(sha256sum -- "$path" | awk '{print $1}')"
    if [[ "$actual" != "$expected" ]]; then
        echo "checksum mismatch for ${path}: expected ${expected}, got ${actual}" >&2
        exit 2
    fi
}

verify_inputs() {
    require_tool sha256sum
    require_tool jq
    require_tool python3
    require_tool fincore
    require_tool /usr/bin/time
    [[ -f "$SQLITE_PATH" ]] || {
        echo "missing SQLite baseline: ${SQLITE_PATH}" >&2
        exit 2
    }
    [[ -f "$HBK_PATH" ]] || {
        echo "missing HBK source: ${HBK_PATH}" >&2
        exit 2
    }
    verify_sha256 "$SQLITE_PATH" "$SQLITE_SHA256"
    verify_sha256 "$HBK_PATH" "$HBK_SHA256"
}

build_harness() {
    cargo build \
        --manifest-path "${REPO_ROOT}/Cargo.toml" \
        --release \
        -p syntax-helper-search \
        --features snapshot-experiment \
        --example measure_hbk_snapshot_scenario \
        --example dump_hbk_snapshot_oracle
}

build_allocation_harness() {
    cargo build \
        --manifest-path "${REPO_ROOT}/Cargo.toml" \
        --target-dir "$ALLOCATION_TARGET" \
        --release \
        -p syntax-helper-search \
        --features snapshot-experiment-alloc \
        --example measure_hbk_snapshot_scenario
}

warm_file() {
    local path="$1"
    [[ -f "$path" ]] || {
        echo "cannot warm missing file: ${path}" >&2
        exit 2
    }
    dd if="$path" of=/dev/null bs=8M status=none
}

evict_file_best_effort() {
    local path="$1"
    [[ -f "$path" ]] || {
        echo "cannot evict missing file: ${path}" >&2
        exit 2
    }
    sync -f "$path"
    python3 -c \
        'import os,sys; fd=os.open(sys.argv[1], os.O_RDONLY); os.posix_fadvise(fd, 0, 0, os.POSIX_FADV_DONTNEED); os.close(fd)' \
        "$path"
}

resident_bytes() {
    local path="$1"
    fincore --bytes --noheadings --output RES -- "$path" | tr -d '[:space:]'
}

prepare_stance() {
    local stance="$1"
    local evict_paths="$2"
    local path
    if [[ "$evict_paths" == "-" ]]; then
        return
    fi
    IFS=':' read -r -a paths <<<"$evict_paths"
    case "$stance" in
        warm)
            for path in "${paths[@]}"; do
                warm_file "$path"
            done
            ;;
        cold-best-effort)
            for path in "${paths[@]}"; do
                evict_file_best_effort "$path"
            done
            ;;
        *)
            echo "unsupported cache stance: ${stance}" >&2
            exit 2
            ;;
    esac
}

resident_json() {
    local evict_paths="$1"
    local result='{}'
    local path
    local bytes
    if [[ "$evict_paths" == "-" ]]; then
        printf '%s\n' "$result"
        return
    fi
    IFS=':' read -r -a paths <<<"$evict_paths"
    for path in "${paths[@]}"; do
        bytes="$(resident_bytes "$path")"
        result="$(
            jq -cn \
                --argjson current "$result" \
                --arg path "$path" \
                --argjson bytes "$bytes" \
                '$current + {($path): $bytes}'
        )"
    done
    printf '%s\n' "$result"
}

harness_commit() {
    if [[ -n "${HBK_BENCH_HARNESS_COMMIT:-}" ]]; then
        printf '%s\n' "$HBK_BENCH_HARNESS_COMMIT"
        return
    fi
    git -C "$REPO_ROOT" log -1 --format=%H -- \
        scripts/benchmark-hbk-snapshot-candidates.sh \
        scripts/summarize-hbk-snapshot-results.py \
        crates/syntax-helper-search/examples/measure_hbk_snapshot_scenario.rs \
        crates/syntax-helper-search/examples/dump_hbk_snapshot_oracle.rs \
        crates/syntax-helper-search/src/snapshot/experiment_allocator.rs \
        crates/syntax-helper-search/src/snapshot/experiment_oracle.rs
}

candidate_commit() {
    if [[ -n "${HBK_BENCH_CANDIDATE_COMMIT:-}" ]]; then
        printf '%s\n' "$HBK_BENCH_CANDIDATE_COMMIT"
        return
    fi
    git -C "$REPO_ROOT" rev-parse HEAD
}

candidate_branch() {
    if [[ -n "${HBK_BENCH_CANDIDATE_BRANCH:-}" ]]; then
        printf '%s\n' "$HBK_BENCH_CANDIDATE_BRANCH"
        return
    fi
    git -C "$REPO_ROOT" branch --show-current
}

run_command() {
    local backend="$1"
    local stance="$2"
    local sample="$3"
    local evict_paths="$4"
    shift 4
    if [[ "${1:-}" != "--" ]]; then
        usage
        exit 2
    fi
    shift
    if [[ "$#" -eq 0 ]]; then
        usage
        exit 2
    fi

    mkdir -p "$LOG_DIR" "$RUN_DIR" "$(dirname -- "$RAW_RESULTS")"
    local run_id="${backend}.${stance}.${sample}"
    local stdout_path="${RUN_DIR}/${run_id}.stdout.json"
    local stderr_path="${LOG_DIR}/${run_id}.stderr.log"
    local time_path="${RUN_DIR}/${run_id}.time.json"
    local residency
    local command_json
    local harness_sha
    local candidate_sha
    local candidate_branch_name
    local kernel
    local architecture
    local rustc_version
    local cargo_version
    local parent_start_unix_ns

    prepare_stance "$stance" "$evict_paths"
    residency="$(resident_json "$evict_paths")"
    harness_sha="$(harness_commit)"
    candidate_sha="$(candidate_commit)"
    candidate_branch_name="$(candidate_branch)"
    command_json="$(printf '%s\n' "$@" | jq -R . | jq -sc .)"
    kernel="$(uname -sr)"
    architecture="$(uname -m)"
    rustc_version="$(rustc --version)"
    cargo_version="$(cargo --version)"

    set +e
    parent_start_unix_ns="$(date +%s%N)"
    HBK_BENCH_PARENT_START_UNIX_NS="$parent_start_unix_ns" \
        LC_ALL=C /usr/bin/time \
        -f '{"elapsed_seconds":%e,"maximum_rss_kib":%M,"minor_page_faults":%R,"major_page_faults":%F}' \
        -o "$time_path" \
        -- "$@" >"$stdout_path" 2>"$stderr_path"
    local command_status=$?
    set -e

    if [[ "$command_status" -ne 0 ]]; then
        jq -cn \
            --arg schema "hbk-snapshot-benchmark-raw-v1" \
            --arg backend "$backend" \
            --arg stance "$stance" \
            --arg dataset "$DATASET_ID" \
            --arg platform_version "$PLATFORM_VERSION" \
            --argjson provider_schema_version "$PROVIDER_SCHEMA_VERSION" \
            --argjson extraction_schema_version "$EXTRACTION_SCHEMA_VERSION" \
            --arg sqlite_sha256 "$SQLITE_SHA256" \
            --arg hbk_sha256 "$HBK_SHA256" \
            --arg kernel "$kernel" \
            --arg architecture "$architecture" \
            --arg rustc "$rustc_version" \
            --arg cargo "$cargo_version" \
            --argjson sample "$sample" \
            --arg harness_commit "$harness_sha" \
            --arg candidate_commit "$candidate_sha" \
            --arg candidate_branch "$candidate_branch_name" \
            --argjson command "$command_json" \
            --argjson resident_bytes_before "$residency" \
            --argjson exit_status "$command_status" \
            '{
                schema: $schema,
                backend: $backend,
                cache_stance: $stance,
                dataset: $dataset,
                platform_version: $platform_version,
                provider_schema_version: $provider_schema_version,
                extraction_schema_version: $extraction_schema_version,
                sqlite_sha256: $sqlite_sha256,
                hbk_sha256: $hbk_sha256,
                build_profile: "release",
                host: {
                    kernel: $kernel,
                    architecture: $architecture,
                    rustc: $rustc,
                    cargo: $cargo
                },
                sample: $sample,
                status: "failed",
                harness_commit: $harness_commit,
                candidate_commit: $candidate_commit,
                candidate_branch: $candidate_branch,
                command: $command,
                resident_bytes_before: $resident_bytes_before,
                exit_status: $exit_status
            }' | tee -a "$RAW_RESULTS"
        echo "benchmark command failed; see ${stderr_path}" >&2
        return "$command_status"
    fi

    jq -e -c . "$stdout_path" >/dev/null
    jq -e -c . "$time_path" >/dev/null
    local inner
    local outer
    inner="$(jq -c . "$stdout_path")"
    outer="$(jq -c . "$time_path")"
    jq -cn \
        --arg schema "hbk-snapshot-benchmark-raw-v1" \
        --arg backend "$backend" \
        --arg stance "$stance" \
        --arg dataset "$DATASET_ID" \
        --arg platform_version "$PLATFORM_VERSION" \
        --argjson provider_schema_version "$PROVIDER_SCHEMA_VERSION" \
        --argjson extraction_schema_version "$EXTRACTION_SCHEMA_VERSION" \
        --arg sqlite_sha256 "$SQLITE_SHA256" \
        --arg hbk_sha256 "$HBK_SHA256" \
        --arg kernel "$kernel" \
        --arg architecture "$architecture" \
        --arg rustc "$rustc_version" \
        --arg cargo "$cargo_version" \
        --argjson sample "$sample" \
        --arg harness_commit "$harness_sha" \
        --arg candidate_commit "$candidate_sha" \
        --arg candidate_branch "$candidate_branch_name" \
        --argjson command "$command_json" \
        --argjson resident_bytes_before "$residency" \
        --argjson measurement "$inner" \
        --argjson process "$outer" \
        '{
            schema: $schema,
            backend: $backend,
            cache_stance: $stance,
            dataset: $dataset,
            platform_version: $platform_version,
            provider_schema_version: $provider_schema_version,
            extraction_schema_version: $extraction_schema_version,
            sqlite_sha256: $sqlite_sha256,
            hbk_sha256: $hbk_sha256,
            build_profile: "release",
            host: {
                kernel: $kernel,
                architecture: $architecture,
                rustc: $rustc,
                cargo: $cargo
            },
            sample: $sample,
            status: "ok",
            harness_commit: $harness_commit,
            candidate_commit: $candidate_commit,
            candidate_branch: $candidate_branch,
            command: $command,
            resident_bytes_before: $resident_bytes_before,
            measurement: $measurement,
            process: $process
        }' | tee -a "$RAW_RESULTS"
}

prepare_cache() {
    local iterations="${1:-$DEFAULT_ITERATIONS}"
    verify_inputs
    [[ -x "$EXAMPLE_BIN" ]] || build_harness
    mkdir -p "$(dirname -- "$PREPARED_CACHE")"
    "$EXAMPLE_BIN" prepare-cache "$SQLITE_PATH" "$PREPARED_CACHE" "$iterations" |
        jq -e -c .
    [[ -f "$PREPARED_CACHE" ]]
    sha256sum -- "$PREPARED_CACHE" >"${PREPARED_CACHE}.sha256"
}

parity_baseline() {
    local parity_dir="${RESULTS_ROOT}/parity"
    local sql_content="${parity_dir}/sql-owned.content-v1.jsonl"
    local sql_lookups="${parity_dir}/sql-owned.lookups-v1.jsonl"
    local cache_content="${parity_dir}/cache-owned.content-v1.jsonl"
    local cache_lookups="${parity_dir}/cache-owned.lookups-v1.jsonl"
    local content_sha
    local lookup_sha
    local harness_sha
    local candidate_sha
    local candidate_branch_name
    local process_index
    local concurrent_content
    local concurrent_lookups
    local -a parity_pids=()

    verify_inputs
    [[ -x "$ORACLE_BIN" ]] || build_harness
    [[ -f "$PREPARED_CACHE" ]] || prepare_cache "$DEFAULT_ITERATIONS"
    mkdir -p "$parity_dir" "$(dirname -- "$RAW_RESULTS")"
    "$ORACLE_BIN" \
        sql-owned "$SQLITE_PATH" "$sql_content" "$sql_lookups"
    "$ORACLE_BIN" \
        cache-owned "$SQLITE_PATH" "$PREPARED_CACHE" "$cache_content" "$cache_lookups"
    jq -e -c . <"$sql_content" >/dev/null
    jq -e -c . <"$sql_lookups" >/dev/null
    jq -e -c . <"$cache_content" >/dev/null
    jq -e -c . <"$cache_lookups" >/dev/null
    cmp -- "$sql_content" "$cache_content"
    cmp -- "$sql_lookups" "$cache_lookups"
    for ((process_index = 1; process_index <= 4; process_index++)); do
        concurrent_content="${parity_dir}/cache-owned.concurrent-${process_index}.content-v1.jsonl"
        concurrent_lookups="${parity_dir}/cache-owned.concurrent-${process_index}.lookups-v1.jsonl"
        "$ORACLE_BIN" \
            cache-owned "$SQLITE_PATH" "$PREPARED_CACHE" \
            "$concurrent_content" "$concurrent_lookups" &
        parity_pids+=("$!")
    done
    for process_index in "${!parity_pids[@]}"; do
        wait "${parity_pids[$process_index]}"
        concurrent_content="${parity_dir}/cache-owned.concurrent-$((process_index + 1)).content-v1.jsonl"
        concurrent_lookups="${parity_dir}/cache-owned.concurrent-$((process_index + 1)).lookups-v1.jsonl"
        cmp -- "$sql_content" "$concurrent_content"
        cmp -- "$sql_lookups" "$concurrent_lookups"
    done
    content_sha="$(sha256sum -- "$sql_content" | awk '{print $1}')"
    lookup_sha="$(sha256sum -- "$sql_lookups" | awk '{print $1}')"
    harness_sha="$(harness_commit)"
    candidate_sha="$(candidate_commit)"
    candidate_branch_name="$(candidate_branch)"
    jq -cn \
        --arg schema "hbk-snapshot-benchmark-raw-v1" \
        --arg dataset "$DATASET_ID" \
        --arg harness_commit "$harness_sha" \
        --arg candidate_commit "$candidate_sha" \
        --arg candidate_branch "$candidate_branch_name" \
        --arg content_sha256 "$content_sha" \
        --arg lookup_sha256 "$lookup_sha" \
        --argjson content_bytes "$(stat -c %s -- "$sql_content")" \
        --argjson lookup_bytes "$(stat -c %s -- "$sql_lookups")" \
        --argjson content_records "$(wc -l <"$sql_content")" \
        --argjson lookup_records "$(wc -l <"$sql_lookups")" \
        '{
            schema: $schema,
            backend: "H0-vs-C0",
            dataset: $dataset,
            scenario: "full-snapshot-parity",
            status: "pass",
            harness_commit: $harness_commit,
            candidate_commit: $candidate_commit,
            candidate_branch: $candidate_branch,
            content_sha256: $content_sha256,
            lookup_sha256: $lookup_sha256,
            content_bytes: $content_bytes,
            lookup_bytes: $lookup_bytes,
            content_records: $content_records,
            lookup_records: $lookup_records,
            concurrent_readers: 4
        }' | tee -a "$RAW_RESULTS"
}

record_parity() {
    local backend="$1"
    local content="$2"
    local lookups="$3"
    local parity_dir="${RESULTS_ROOT}/parity"
    local baseline_content="${parity_dir}/sql-owned.content-v1.jsonl"
    local baseline_lookups="${parity_dir}/sql-owned.lookups-v1.jsonl"
    local status=pass
    local harness_sha
    local candidate_sha
    local candidate_branch_name

    [[ -f "$baseline_content" && -f "$baseline_lookups" ]] || parity_baseline >/dev/null
    jq -e -c . <"$content" >/dev/null
    jq -e -c . <"$lookups" >/dev/null
    if ! cmp -- "$baseline_content" "$content" >/dev/null ||
        ! cmp -- "$baseline_lookups" "$lookups" >/dev/null; then
        status=fail
    fi
    harness_sha="$(harness_commit)"
    candidate_sha="$(candidate_commit)"
    candidate_branch_name="$(candidate_branch)"
    jq -cn \
        --arg schema "hbk-snapshot-benchmark-raw-v1" \
        --arg backend "$backend" \
        --arg dataset "$DATASET_ID" \
        --arg status "$status" \
        --arg harness_commit "$harness_sha" \
        --arg candidate_commit "$candidate_sha" \
        --arg candidate_branch "$candidate_branch_name" \
        --arg content_sha256 "$(sha256sum -- "$content" | awk '{print $1}')" \
        --arg lookup_sha256 "$(sha256sum -- "$lookups" | awk '{print $1}')" \
        '{
            schema: $schema,
            backend: $backend,
            dataset: $dataset,
            scenario: "full-snapshot-parity",
            status: $status,
            harness_commit: $harness_commit,
            candidate_commit: $candidate_commit,
            candidate_branch: $candidate_branch,
            content_sha256: $content_sha256,
            lookup_sha256: $lookup_sha256
        }' | tee -a "$RAW_RESULTS"
    [[ "$status" == "pass" ]]
}

allocation_baseline() {
    local backend="$1"
    local runs="${2:-$DEFAULT_ALLOCATION_RUNS}"
    local iterations="${3:-$DEFAULT_ITERATIONS}"
    local sample
    local cache_path
    local stdout_path
    local stderr_path
    local measurement
    local harness_sha
    local candidate_sha
    local candidate_branch_name

    verify_inputs
    [[ -x "$ALLOCATION_EXAMPLE_BIN" ]] || build_allocation_harness
    [[ -f "$PREPARED_CACHE" ]] || prepare_cache "$iterations"
    mkdir -p "$RUN_DIR" "$LOG_DIR" "$(dirname -- "$RAW_RESULTS")"
    harness_sha="$(harness_commit)"
    candidate_sha="$(candidate_commit)"
    candidate_branch_name="$(candidate_branch)"

    for ((sample = 1; sample <= runs; sample++)); do
        stdout_path="${RUN_DIR}/allocation.${backend}.${sample}.json"
        stderr_path="${LOG_DIR}/allocation.${backend}.${sample}.stderr.log"
        case "$backend" in
            sql-owned)
                warm_file "$SQLITE_PATH"
                "$ALLOCATION_EXAMPLE_BIN" \
                    sql-owned "$SQLITE_PATH" "$iterations" \
                    >"$stdout_path" 2>"$stderr_path"
                ;;
            cache-owned)
                cache_path="${RUN_DIR}/allocation-cache.${sample}.bin"
                cp --reflink=auto --preserve=mode,timestamps -- "$PREPARED_CACHE" "$cache_path"
                warm_file "$SQLITE_PATH"
                warm_file "$cache_path"
                "$ALLOCATION_EXAMPLE_BIN" \
                    cache-owned "$SQLITE_PATH" "$cache_path" "$iterations" \
                    >"$stdout_path" 2>"$stderr_path"
                ;;
            *)
                echo "unsupported allocation backend: ${backend}" >&2
                exit 2
                ;;
        esac
        measurement="$(jq -e -c 'select(.allocations.enabled == true)' "$stdout_path")"
        jq -cn \
            --arg schema "hbk-snapshot-benchmark-raw-v1" \
            --arg backend "$backend" \
            --arg dataset "$DATASET_ID" \
            --arg harness_commit "$harness_sha" \
            --arg candidate_commit "$candidate_sha" \
            --arg candidate_branch "$candidate_branch_name" \
            --argjson sample "$sample" \
            --argjson measurement "$measurement" \
            '{
                schema: $schema,
                backend: $backend,
                dataset: $dataset,
                cache_stance: "warm",
                scenario: "allocation-profile",
                instrumentation: "counting-system-global-allocator",
                sample: $sample,
                status: "ok",
                harness_commit: $harness_commit,
                candidate_commit: $candidate_commit,
                candidate_branch: $candidate_branch,
                measurement: $measurement
            }' | tee -a "$RAW_RESULTS"
    done
}

run_baseline_backend() {
    local backend="$1"
    local stance="$2"
    local sample="$3"
    local iterations="$4"
    case "$backend" in
        sql-owned)
            run_command \
                "$backend" "$stance" "$sample" "$SQLITE_PATH" -- \
                "$EXAMPLE_BIN" sql-owned "$SQLITE_PATH" "$iterations"
            ;;
        cache-owned)
            mkdir -p "$RUN_DIR"
            local run_cache="${RUN_DIR}/current-cache.${stance}.${sample}.bin"
            cp --reflink=auto --preserve=mode,timestamps -- "$PREPARED_CACHE" "$run_cache"
            local prepared_sha
            local run_sha
            prepared_sha="$(sha256sum -- "$PREPARED_CACHE" | awk '{print $1}')"
            run_sha="$(sha256sum -- "$run_cache" | awk '{print $1}')"
            if [[ "$prepared_sha" != "$run_sha" ]]; then
                echo "per-run cache copy checksum mismatch" >&2
                exit 2
            fi
            run_command \
                "$backend" "$stance" "$sample" "${SQLITE_PATH}:${run_cache}" -- \
                "$EXAMPLE_BIN" cache-owned "$SQLITE_PATH" "$run_cache" "$iterations"
            ;;
        *)
            echo "unsupported baseline backend: ${backend}" >&2
            exit 2
            ;;
    esac
}

run_baselines() {
    local stance="$1"
    local runs="${2:-$DEFAULT_RUNS}"
    local iterations="${3:-$DEFAULT_ITERATIONS}"
    local sample
    verify_inputs
    [[ -x "$EXAMPLE_BIN" ]] || build_harness
    [[ -f "$PREPARED_CACHE" ]] || prepare_cache "$iterations"

    if [[ "$stance" == "warm" ]]; then
        for ((sample = 1; sample <= WARMUP_RUNS; sample++)); do
            prepare_stance warm "$SQLITE_PATH"
            "$EXAMPLE_BIN" sql-owned "$SQLITE_PATH" "$iterations" >/dev/null
            prepare_stance warm "${SQLITE_PATH}:${PREPARED_CACHE}"
            "$EXAMPLE_BIN" cache-owned "$SQLITE_PATH" "$PREPARED_CACHE" "$iterations" >/dev/null
        done
    elif [[ "$stance" != "cold-best-effort" ]]; then
        echo "unsupported cache stance: ${stance}" >&2
        exit 2
    fi

    for ((sample = 1; sample <= runs; sample++)); do
        if ((sample % 2 == 1)); then
            run_baseline_backend sql-owned "$stance" "$sample" "$iterations"
            run_baseline_backend cache-owned "$stance" "$sample" "$iterations"
        else
            run_baseline_backend cache-owned "$stance" "$sample" "$iterations"
            run_baseline_backend sql-owned "$stance" "$sample" "$iterations"
        fi
    done
}

read_smaps_json() {
    local pid="$1"
    awk '
        BEGIN {
            rss = 0; pss = 0; private_kib = 0; shared_kib = 0; anonymous_kib = 0
        }
        $1 == "Rss:" { rss = $2 }
        $1 == "Pss:" { pss = $2 }
        $1 == "Private_Clean:" || $1 == "Private_Dirty:" { private_kib += $2 }
        $1 == "Shared_Clean:" || $1 == "Shared_Dirty:" { shared_kib += $2 }
        $1 == "Anonymous:" { anonymous_kib = $2 }
        END {
            printf "{\"pid\":%d,\"rss_kib\":%d,\"pss_kib\":%d,\"private_kib\":%d,\"shared_kib\":%d,\"anonymous_kib\":%d}\n",
                pid, rss, pss, private_kib, shared_kib, anonymous_kib
        }
    ' pid="$pid" "/proc/${pid}/smaps_rollup"
}

multi_reader_baseline_once() {
    local backend="$1"
    local sample="$2"
    local iterations="$3"
    local hold_ms=10000
    local multi_dir="${RUN_DIR}/multi-reader.${backend}.${sample}"
    local process_index
    local ready_count
    local poll
    local pid
    local per_process='[]'
    local cache_path
    local result
    local total_pss
    local total_rss
    local total_private
    local total_shared
    local total_anonymous
    local harness_sha
    local candidate_sha
    local candidate_branch_name
    local -a pids=()
    local -a ready_files=()

    verify_inputs
    [[ -x "$EXAMPLE_BIN" ]] || build_harness
    [[ -f "$PREPARED_CACHE" ]] || prepare_cache "$iterations"
    mkdir -p "$multi_dir" "$(dirname -- "$RAW_RESULTS")"

    warm_file "$SQLITE_PATH"
    if [[ "$backend" == "cache-owned" ]]; then
        for ((process_index = 1; process_index <= 4; process_index++)); do
            cache_path="${multi_dir}/current-cache.${process_index}.bin"
            cp --reflink=auto --preserve=mode,timestamps -- "$PREPARED_CACHE" "$cache_path"
            warm_file "$cache_path"
        done
    elif [[ "$backend" != "sql-owned" ]]; then
        echo "unsupported multi-reader backend: ${backend}" >&2
        exit 2
    fi

    for ((process_index = 1; process_index <= 4; process_index++)); do
        local ready_file="${multi_dir}/ready.${process_index}"
        local stdout_file="${multi_dir}/stdout.${process_index}.json"
        local stderr_file="${multi_dir}/stderr.${process_index}.log"
        ready_files+=("$ready_file")
        if [[ -e "$ready_file" ]]; then
            rm -- "$ready_file"
        fi
        if [[ "$backend" == "sql-owned" ]]; then
            HBK_BENCH_HOLD_MS="$hold_ms" \
                HBK_BENCH_READY_FILE="$ready_file" \
                "$EXAMPLE_BIN" sql-owned "$SQLITE_PATH" "$iterations" \
                >"$stdout_file" 2>"$stderr_file" &
        else
            cache_path="${multi_dir}/current-cache.${process_index}.bin"
            HBK_BENCH_HOLD_MS="$hold_ms" \
                HBK_BENCH_READY_FILE="$ready_file" \
                "$EXAMPLE_BIN" cache-owned "$SQLITE_PATH" "$cache_path" "$iterations" \
                >"$stdout_file" 2>"$stderr_file" &
        fi
        pids+=("$!")
    done

    ready_count=0
    for ((poll = 0; poll < 200; poll++)); do
        ready_count=0
        for ready_file in "${ready_files[@]}"; do
            [[ -f "$ready_file" ]] && ((ready_count += 1))
        done
        [[ "$ready_count" -eq 4 ]] && break
        sleep 0.1
    done
    if [[ "$ready_count" -ne 4 ]]; then
        echo "multi-reader processes did not reach hold point" >&2
        for pid in "${pids[@]}"; do
            kill "$pid" 2>/dev/null || true
        done
        for pid in "${pids[@]}"; do
            wait "$pid" 2>/dev/null || true
        done
        exit 1
    fi

    for pid in "${pids[@]}"; do
        result="$(read_smaps_json "$pid")"
        per_process="$(jq -cn --argjson rows "$per_process" --argjson row "$result" '$rows + [$row]')"
    done
    total_pss="$(jq '[.[].pss_kib] | add' <<<"$per_process")"
    total_rss="$(jq '[.[].rss_kib] | add' <<<"$per_process")"
    total_private="$(jq '[.[].private_kib] | add' <<<"$per_process")"
    total_shared="$(jq '[.[].shared_kib] | add' <<<"$per_process")"
    total_anonymous="$(jq '[.[].anonymous_kib] | add' <<<"$per_process")"
    harness_sha="$(harness_commit)"
    candidate_sha="$(candidate_commit)"
    candidate_branch_name="$(candidate_branch)"

    jq -cn \
        --arg schema "hbk-snapshot-benchmark-raw-v1" \
        --arg backend "$backend" \
        --arg dataset "$DATASET_ID" \
        --arg harness_commit "$harness_sha" \
        --arg candidate_commit "$candidate_sha" \
        --arg candidate_branch "$candidate_branch_name" \
        --argjson sample "$sample" \
        --argjson per_process "$per_process" \
        --argjson total_pss_kib "$total_pss" \
        --argjson total_rss_kib "$total_rss" \
        --argjson total_private_kib "$total_private" \
        --argjson total_shared_kib "$total_shared" \
        --argjson total_anonymous_kib "$total_anonymous" \
        '{
            schema: $schema,
            backend: $backend,
            dataset: $dataset,
            cache_stance: "warm",
            scenario: "aggregate-four-reader-pss",
            sample: $sample,
            status: "ok",
            harness_commit: $harness_commit,
            candidate_commit: $candidate_commit,
            candidate_branch: $candidate_branch,
            per_process: $per_process,
            aggregate: {
                rss_kib: $total_rss_kib,
                pss_kib: $total_pss_kib,
                private_kib: $total_private_kib,
                shared_kib: $total_shared_kib,
                anonymous_kib: $total_anonymous_kib
            }
        }' | tee -a "$RAW_RESULTS"

    for pid in "${pids[@]}"; do
        wait "$pid"
    done
}

multi_reader_baselines() {
    local backend="$1"
    local runs="${2:-$DEFAULT_ALLOCATION_RUNS}"
    local iterations="${3:-$DEFAULT_ITERATIONS}"
    local sample
    for ((sample = 1; sample <= runs; sample++)); do
        multi_reader_baseline_once "$backend" "$sample" "$iterations"
    done
}

print_paths() {
    jq -cn \
        --arg sqlite "$SQLITE_PATH" \
        --arg hbk "$HBK_PATH" \
        --arg prepared_cache "$PREPARED_CACHE" \
        --arg raw_results "$RAW_RESULTS" \
        --arg result_root "$RESULTS_ROOT" \
        '{
            sqlite: $sqlite,
            hbk: $hbk,
            prepared_cache: $prepared_cache,
            raw_results: $raw_results,
            result_root: $result_root
        }'
}

main() {
    local action="${1:-}"
    case "$action" in
        verify)
            verify_inputs
            ;;
        build)
            build_harness
            ;;
        prepare-cache)
            shift
            prepare_cache "$@"
            ;;
        parity-baseline)
            parity_baseline
            ;;
        record-parity)
            shift
            [[ "$#" -eq 3 ]] || {
                usage
                exit 2
            }
            record_parity "$@"
            ;;
        allocation-baseline)
            shift
            [[ "$#" -ge 1 ]] || {
                usage
                exit 2
            }
            allocation_baseline "$@"
            ;;
        baseline)
            shift
            [[ "$#" -ge 1 ]] || {
                usage
                exit 2
            }
            run_baselines "$@"
            ;;
        run-command)
            shift
            [[ "$#" -ge 6 ]] || {
                usage
                exit 2
            }
            run_command "$@"
            ;;
        multi-reader-baseline)
            shift
            [[ "$#" -ge 1 ]] || {
                usage
                exit 2
            }
            multi_reader_baselines "$@"
            ;;
        paths)
            print_paths
            ;;
        *)
            usage
            exit 2
            ;;
    esac
}

main "$@"
