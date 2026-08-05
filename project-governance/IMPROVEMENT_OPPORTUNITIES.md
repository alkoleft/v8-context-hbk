# Improvement Opportunity Registry

This registry preserves potentially useful improvements that have not been
accepted as work. It is supporting discovery evidence, not a requirements
source, roadmap, approved design, or implementation ledger. Canonical
requirements and accepted change scope live in OpenSpec; implementation starts
only after an opportunity is promoted into a separate apply-ready OpenSpec
change.

The registry neither ranks entries nor assigns delivery responsibility. Entry
order carries no sequencing meaning. A promoted entry remains here only as
decision provenance while its linked OpenSpec change owns all work state.

## Entry contract

Every record has a stable `IMP-NNNN` identifier and these fields:

- **Categories** — applicable improvement dimensions, not a closed taxonomy;
- **Evidence disposition** — exactly one of `captured`, `needs-evidence`,
  `validated-candidate`, `conditional-candidate`, `promoted`, `rejected`, or
  `superseded`;
- **Affected areas** — the contracts or contexts in which the idea might fit;
- **Opportunity** — the observed problem or unclaimed potential;
- **Hypothesis** — the smallest design claim worth evaluating;
- **Expected value** — the outcome that would justify promotion;
- **Evidence and origin** — reproducible measurements or the concrete source
  that warrants retaining the idea;
- **Constraints and trade-offs** — known reasons the idea may not fit;
- **Promotion trigger** — evidence that would justify proposing accepted work;
- **Review trigger** — events that require rechecking the record;
- **OpenSpec relationship** — the promoted change, or an explicit statement
  that no change owns implementation;
- **Last reviewed** — the date on which evidence and disposition were checked.

The evidence disposition describes knowledge or decision maturity, never
execution progress. Records contain no work checklist, implementation
sequence, delivery rank, schedule, or person assignment. Rejected and
superseded records remain with their rationale so the same idea is not
reopened without new evidence.

## IMP-0001 — Snapshot-local ID-only hash index for incoming-string lookup

- **Categories:** performance, architecture
- **Evidence disposition:** `validated-candidate`
- **Affected areas:** `syntax-helper-search` frozen snapshot string lookup;
  potentially analogous private typed lookup indexes in downstream contexts
- **Opportunity:** the current sorted-text lookup preserves one string owner
  and stable dense `StringId` values, but binary search performs substantially
  more work than a hash probe when the caller supplies `&str`.
- **Hypothesis:** retain the existing `Vec<String>` as the sole text owner and
  keep `StringId == vector index`; add a pre-sized frozen hash structure whose
  entries contain only `StringId`. A lookup hashes the incoming normalized
  string, then verifies equality through `strings[id]`. Reverse lookup and
  serialized identity remain unchanged.
- **Expected value:** reduce `&str -> StringId` latency without duplicating
  string payload, changing public IDs, or introducing a second identity owner.
- **Evidence and origin:** the `experiment/string-interner-bench` result at
  commit `cf90c0f` archives the benchmark produced by measurement commit
  `b446d3ab347098d9b59440b483c184f75152fb7e`. On 71,073 unique exact strings,
  median exact-hit lookup fell from 466 to 93 ns/query and pre-sized build from
  25.54 to 14.45 ms. On the authoritative 48,355-query raw-name replay,
  normalize-plus-hit lookup fell from 775 to 513 ns/query, ratio `0.643` with
  95% bootstrap CI `[0.634, 0.688]`. Comparable standalone retained memory was
  6.45 MiB versus 6.10 MiB for sorted text, an approximately 0.35 MiB index
  increment while reusing the existing string owner. All measured adapters had
  identical dense IDs, reverse resolution, checksums, and zero mismatches.
- **Constraints and trade-offs:** normalization alone measured 444 ns/query
  and accounts for about 87% of the raw-hit time with this index, so replacing
  lookup cannot remove the dominant raw-input cost. The index still adds build
  time, retained bytes, hash-policy choice, and collision verification. The
  experiment is a private dictionary benchmark, not proof of an end-to-end
  typed consumer bottleneck. It does not authorize an X1 layout or public ID
  change.
- **Promotion trigger:** a profile of a concrete typed consumer identifies
  repeated incoming-string-to-ID lookup as material, and an integrated
  snapshot-local proof demonstrates lookup parity, raw end-to-end improvement,
  acceptable build/retained resources, reuse of the existing string owner, and
  no serialized or public identity change unless that separate contract is
  explicitly proposed.
- **Review trigger:** snapshot string ownership, normalization behavior, X1
  mapping requirements, or the representative lookup workload changes; or a
  typed end-to-end profile becomes available.
- **OpenSpec relationship:** not promoted. The OpenSpec change that established
  this registry records evidence only and owns no index implementation.
- **Last reviewed:** 2026-08-05

Adjacent applicability is deliberately narrower than sharing HBK symbols.
`v8-context` has scan-based BSL and SDBL lookup seams where a local typed
ID-only index may help after local profiling. `v8-context-metadata` already
uses interned qualified-name segments and SQLite exact indexes; its plausible
separate seam is a typed source-root/path index, not reuse of this snapshot ID
space.

## IMP-0002 — `string-interner` as the sole final snapshot string owner

- **Categories:** performance, architecture, dependency evaluation
- **Evidence disposition:** `conditional-candidate`
- **Affected areas:** snapshot materialization and exact incoming-string lookup
  only if the final string ownership contract can change intentionally
- **Opportunity:** a maintained interning crate can provide fast dense private
  symbols and construction, but only has architectural value here if it
  replaces the current final string owner instead of sitting beside it or
  being copied into it.
- **Hypothesis:** pin `string-interner` and make its frozen dictionary the sole
  final owner of snapshot strings, keeping symbols generation-local and
  adapting snapshot/X1 boundaries through an explicitly approved ownership
  design. Do not retain it as a sidecar beside `Vec<String>`.
- **Expected value:** retain the measured fast exact lookup and construction
  while avoiding duplicate payload ownership and preserving typed external
  identities at the boundary.
- **Evidence and origin:** the same archived experiment at commit `cf90c0f`
  found `string-interner 0.20.0` to be the strongest library candidate. Its
  native configuration measured 70 ns/query for exact hits, 553 ns/query for
  raw normalize-plus-hit, 4.93 ms for pre-sized exact construction, and
  6.59 MiB retained as a standalone sole-owner dictionary. However, it did not
  establish a raw-lookup advantage over the ID-only control: the native
  interner/control ratio was `1.078`, CI `[0.915, 1.331]`, and the common-hasher
  diagnostic was `0.998`, CI `[0.967, 1.062]`.
- **Constraints and trade-offs:** the current final contract requires
  `Vec<String>` with dense `StringId` values. Public library APIs cannot move
  owned strings into that vector, so the builder-only experiment copied them:
  finalization improved from 43.67 to 9.32 ms but peak live memory grew from
  9.68 to 12.42 MiB (`+28.3%`), failing the experiment's 10% peak budget.
  Retaining both owners would spend more memory without a proven raw-lookup
  advantage. Library symbols must not become persistent, cross-generation, or
  cross-repository identities.
- **Promotion trigger:** an accepted ownership design permits the interner to
  replace the final string vector, or explicitly accepts the builder-only peak
  trade-off, and a full typed snapshot/X1 benchmark proves material benefit
  over the ID-only control without duplicate string ownership or leaked
  library symbols.
- **Review trigger:** the snapshot/X1 ownership contract changes; the selected
  crate exposes a move-out/frozen representation that removes copying; the
  resource budget changes; or a new end-to-end benchmark changes the control
  comparison.
- **OpenSpec relationship:** not promoted. No production dependency, snapshot
  ownership change, identity change, or X1 change is authorized.
- **Last reviewed:** 2026-08-05

For adjacent repositories the fit is conditional and local. `v8-context` may
benefit only where one parsed-module or prepared-context owner can make an
interner its sole name store and profiling proves repeated string-to-symbol
lookup. `v8-context-metadata` already uses `ArcIntern<str>` for qualified-name
segments and persistent SQLite indexes, so adding another interner currently
has low fit and risks competing identity owners.
