# All-HBK smoke report

Task: T10 all-HBK smoke report.

Date: 2026-04-29.

Repository revision under test: local working tree after T9, before marking T10 complete.

Target platform: `/opt/1cv8/x86_64/8.5.1.1150/`.

## Commands

Run from repository root.

```bash
cargo build
set -u
platform_dir=/opt/1cv8/x86_64/8.5.1.1150
out=target/acceptance/t10/all-hbk-smoke.tsv
mkdir -p "$(dirname "$out")"
printf 'file\tinspect_exit\ttoc_exit\tinspect_summary\ttoc_summary\n' > "$out"
for file in $(find "$platform_dir" -maxdepth 1 -type f -name '*.hbk' | sort); do
  inspect_log="target/acceptance/t10/$(basename "$file").inspect.log"
  toc_log="target/acceptance/t10/$(basename "$file").toc.log"
  if target/debug/v8-context-hbk inspect "$file" >"$inspect_log" 2>&1; then inspect_exit=0; else inspect_exit=$?; fi
  if target/debug/v8-context-hbk toc "$file" --format json >"$toc_log" 2>&1; then toc_exit=0; else toc_exit=$?; fi
  inspect_summary=$(tr '\n\t' '  ' <"$inspect_log" | sed 's/  */ /g' | cut -c1-180)
  toc_summary=$(tr '\n\t' '  ' <"$toc_log" | sed 's/  */ /g' | cut -c1-180)
  printf '%s\t%s\t%s\t%s\t%s\n' "$file" "$inspect_exit" "$toc_exit" "$inspect_summary" "$toc_summary" >> "$out"
done
awk -F '\t' 'NR>1 {total++; if ($2==0) inspect_ok++; else inspect_fail++; if ($3==0) toc_ok++; else toc_fail++; if ($2!=0 || $3!=0) failures++} END {printf "total=%d inspect_ok=%d inspect_fail=%d toc_ok=%d toc_fail=%d files_with_failures=%d\n", total, inspect_ok, inspect_fail, toc_ok, toc_fail, failures}' "$out"
awk -F '\t' 'NR>1 && ($2!=0 || $3!=0) {print}' "$out"
```

The loop ran `inspect` and `toc --format json` once for every path returned by the `find` command.
Temporary command logs were written under `target/acceptance/t10/` and are not checked in.

## Summary

| Metric | Count |
| --- | ---: |
| HBK files discovered | 116 |
| `inspect` successes | 116 |
| `inspect` fatal failures | 0 |
| `toc --format json` successes | 116 |
| `toc --format json` fatal failures | 0 |
| Files with unsupported structures | 0 |

Locale suffix coverage:

| Suffix | Count |
| --- | ---: |
| `_hu.hbk` | 36 |
| `_root.hbk` | 40 |
| `_ru.hbk` | 40 |

## Per-file results

| HBK file | `inspect` exit | `toc --format json` exit | Result |
| --- | ---: | ---: | --- |
| `/opt/1cv8/x86_64/8.5.1.1150/1cv8_hu.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/1cv8_root.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/1cv8_ru.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/accntui_hu.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/accntui_root.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/accntui_ru.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/basicui_hu.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/basicui_root.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/basicui_ru.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/bpui_hu.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/bpui_root.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/bpui_ru.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/calcui_hu.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/calcui_root.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/calcui_ru.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/chartui_hu.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/chartui_root.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/chartui_ru.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/chdbfl_hu.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/chdbfl_root.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/chdbfl_ru.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/config_hu.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/config_root.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/config_ru.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/dcsui_hu.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/dcsui_root.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/dcsui_ru.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/debug_hu.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/debug_root.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/debug_ru.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/devtool_hu.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/devtool_root.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/devtool_ru.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/dhistui_hu.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/dhistui_root.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/dhistui_ru.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/dsgncmd_hu.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/dsgncmd_root.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/dsgncmd_ru.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/dsgnfrm_hu.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/dsgnfrm_root.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/dsgnfrm_ru.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/ecsui_hu.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/ecsui_root.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/ecsui_ru.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/edbui_hu.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/edbui_root.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/edbui_ru.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/extui_hu.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/extui_root.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/extui_ru.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/fmtdui_hu.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/fmtdui_root.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/fmtdui_ru.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/frame_hu.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/frame_root.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/frame_ru.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/frntend_hu.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/frntend_root.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/frntend_ru.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/helpui_hu.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/helpui_root.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/helpui_ru.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/htmlui_hu.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/htmlui_root.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/htmlui_ru.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/integui_hu.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/integui_root.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/integui_ru.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/mapui_hu.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/mapui_root.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/mapui_ru.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/mngbase_hu.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/mngbase_root.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/mngbase_ru.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/mngcln_hu.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/mngcln_root.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/mngcln_ru.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/mngdsgn_hu.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/mngdsgn_root.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/mngdsgn_ru.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/mngui_hu.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/mngui_root.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/mngui_ru.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/moxelui_hu.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/moxelui_root.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/moxelui_ru.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/pdfui_hu.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/pdfui_root.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/pdfui_ru.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/perform_hu.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/perform_root.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/perform_ru.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/pictedt_hu.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/pictedt_root.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/pictedt_ru.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/plnnrui_hu.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/plnnrui_root.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/plnnrui_ru.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/richui_hu.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/richui_root.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/richui_ru.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/schemui_hu.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/schemui_root.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/schemui_ru.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/shclang_root.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/shclang_ru.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/shcntx_root.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/shcntx_ru.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/shlang_root.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/shlang_ru.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/shquery_root.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/shquery_ru.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/txtedui_hu.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/txtedui_root.hbk` | 0 | 0 | ok |
| `/opt/1cv8/x86_64/8.5.1.1150/txtedui_ru.hbk` | 0 | 0 | ok |

## Unsupported structures and follow-up tasks

No fatal failures or unsupported structures were observed by the generic container/book/TOC smoke commands.
No T10 follow-up implementation tasks are needed from this pass.

## Verification

Required verification for T10:

- All-HBK smoke report exists and references the exact commands used.
- `cargo test`.
- `git diff --check`.
