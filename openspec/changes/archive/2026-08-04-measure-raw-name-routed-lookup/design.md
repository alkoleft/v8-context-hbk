## Context

The current owned lookup normalizes a raw name and binary-searches an index
whose key is a `StringId`; comparisons dereference that key through the
snapshot string table. The proposed replacement has a different ownership
model:

```text
raw &str -> normalize_lookup_key -> route -> binary search text -> typed ID
```

For primary type names the intern table and identity table are the same table:
each unique normalized name occurs once, and its row index is `TypeId`. There
is no `StringId -> TypeId` correspondence. Aliases remain search-only keys that
point to the canonical typed ID.

The experiment remains private and feature-gated. It evaluates the bilingual
convention only: eligible primary queries begin with a non-ASCII-Latin
character and eligible aliases begin with an ASCII-Latin letter. English-only
primary names without a Russian primary are outside the hypothesis.

Applicable cross-project invariants are AIR-001 through AIR-003. The HBK
provider remains the owner. No shared registry or analyzer-side entity mirror
is introduced.

## Goals / Non-Goals

**Goals:**

- Give the current reference and typed-table candidate the same raw `&str`.
- Include `normalize_lookup_key` in both timed paths.
- Remove `StringId` from every candidate table, ID, query and lookup operation.
- Make a primary table row index equal its family-specific interned-name ID;
  for types that ordinal is the complete `TypeId`.
- Route an ASCII-Latin normalized first character to aliases and every other
  first character to primaries, with no fallback search.
- Differentially validate eligible primary, alias, miss and owner-isolation
  behavior for type, callable and property families.
- Measure same-run latency, construction allocations and retained storage on
  the frozen 8.3.27.1859 corpus.

**Non-Goals:**

- Supporting English-only primary names without a Russian primary.
- Changing production snapshot formation, normalization, indexes, public APIs,
  snapshot/X1 schemas or current provider IDs.
- Adding a general registry, generation field, cache, fallback search or
  language detector.
- Making table ordinals stable across different HBK snapshots or versions.
- Adding signatures or parameter IDs.

## Decisions

### The primary type-name table is the TypeId allocator

The candidate SHALL own one lexically sorted table of unique normalized primary
type names. `TypeId` SHALL be the typed ordinal of a row in that table. A
primary lookup SHALL binary-search the table and construct `TypeId` directly
from the found row index. It SHALL NOT return or resolve an intermediate string
token.

```text
primary_type_names[TypeId] = normalized primary name
```

Sorting fixes deterministic IDs within one immutable HBK snapshot. Adding or
renaming a type may shift later ordinals in a different snapshot; cross-version
ID stability is not claimed.

The temporary experiment SHALL discard duplicate normalized primary rows before
ordinal assignment and report their count. This is explicitly temporary
formation logic: the accepted invariant is that HBK primary names are unique,
and future formation work must reject or resolve source extensions rather than
silently discard them.

### Callable and property IDs use owner plus a family-specific name ordinal

Callable/property primary names are not globally unique as entities. Each
family SHALL therefore intern its unique normalized primary-name strings in its
own sorted table. The name row ordinal is combined with the canonical owner to
form `CallableId` or `PropertyId`:

```text
CallableId = (OwnerId, callable-name ordinal)
PropertyId = (OwnerId, property-name ordinal)
```

`OwnerId` distinguishes global context, type and enum owners using the typed
identity already assigned by the experiment. Repeated primary text such as an
`Добавить` method is stored once in the callable-name table but yields distinct
callable identities for distinct owners. No common string-token type exists.

Each scoped family additionally SHALL retain one primary search vector of
completed IDs, ordered by `(OwnerId, family_names[id.name])`. The vector is the
owner-membership index: binary search compares the requested `(owner, text)`
against that ordered projection and returns the already completed ID. The name
table allocates the ordinal; it does not by itself prove that an owner declares
the name. Lookup under an owner that lacks the completed ID SHALL be missing
even when another owner uses the same name ordinal.

This is one text search with name-table indirection during comparison, not a
text-to-token search followed by a membership search. The primary vector stores
only the completed ID and therefore does not repeat the owner or primary text.

### Aliases are separate text-to-identity indexes

Each family SHALL retain a lexically sorted alias index. A type alias maps
directly to `TypeId`; callable/property aliases include owner in their sorted
search key and map to the completed composite ID. Aliases SHALL NOT allocate
identity and SHALL NOT be inserted into the primary table.

### Candidate input and retained state contain no StringId

The source-adapter phase SHALL read each legacy `HbkNameView` and immediately
resolve its handles to borrowed raw `&str`. It SHALL project those strings into
experiment source rows that contain only borrowed text and source owner
ordinals. In parallel, baseline-only rows SHALL retain the existing normalized
snapshot `StringId` keys needed to execute the current lookup mechanics. A
transient borrowed `normalized &str -> existing StringId` preparation map may
project `HbkNameView` raw handles onto those already owned index keys; it SHALL
be dropped before construction and lookup measurement. The experimental
candidate SHALL copy each retained normalized name once into its family table
or alias index. Legacy `StringId` SHALL not enter projected candidate source
rows or raw query rows. Candidate rows, typed IDs, aliases, sorting, lookup and
measurement closures SHALL not contain, construct or compare `StringId`.

The same raw query text SHALL be passed to the current reference and candidate.
Both SHALL normalize within the measured call. The reference may use its
current production representation; the candidate SHALL search owned normalized
text directly.

### Route by the first normalized ASCII-Latin character

After normalization, `first().is_ascii_alphabetic()` SHALL select only the
alias index; every other first character, including an empty key, SHALL select
only the primary table. There is no fallback.

Correctness and timing query sets SHALL contain:

- non-ASCII-Latin primary names;
- ASCII-Latin aliases;
- one ASCII-Latin and one non-ASCII-Latin missing raw input;
- owner-isolation queries satisfying the same script rule.

English-only primary names and non-ASCII aliases SHALL be excluded and counted.
Missing inputs SHALL be borrowed raw strings from existing snapshot data whose
normalized text is proven absent across all retained primary and alias names;
no reverse map or synthetic ID is retained.

### Correctness and timing evidence remain separate

Before timing, transient oracle state SHALL compare the candidate with the
current raw lookup by canonical source entity. It SHALL be dropped before
timing. Both lookup lanes SHALL consume the identical raw query sequence and
sample count. Each lane SHALL black-box and checksum its native result IDs so
neither is charged a benchmark-only remapping lookup; equality is established
by the pre-timing differential mismatch count, not by comparing numeric
checksums from different ID layouts. The report SHALL label the baseline as
current raw lookup and publish family coverage, duplicate counts, semantic
mismatches, end-to-end medians, construction allocations and retained bytes.
Earlier prepared-key measurements remain historical and SHALL not be relabelled.

## Structure impact

Searched owners and consumers:

- experiment: `SourceTypeRow`, `SourceMemberRow`, `CanonicalRow`, `Query`,
  `QuerySets`, `PrimaryAliasLookup`, `run_family`, `project_name`, allocation
  observer and existing prepared-key tests;
- production: `HbkFactSnapshot::strings`, `StringId`, `HbkNameView`,
  `NameLookup`, `OwnerNameLookup`, `matching_range`, `normalize_lookup_key`,
  `platform_types_by_name`, materialization and X1 lookup;
- evidence: frozen runner, canonical capability and archived measurements. No
  frontend, schema, generator, export or analyzer consumer exists for this
  private experiment.

Reused: existing raw provider names and normalized snapshot index keys as the
current-baseline boundary, normalization, canonical source rows,
family-specific typed experimental IDs, query classification, differential
oracle and allocation observer.

Changed: the private candidate representation and frozen runner become raw-name
end-to-end. Added: the type primary-name table; callable/property name tables;
one completed-ID primary membership vector for each scoped family; composite
owner IDs; alias indexes; routing predicate; and parallel baseline-only rows
that point at existing normalized snapshot keys. A transient source-preparation
map resolves raw names to those existing legacy keys and is dropped before all
measurements. Deleted from the candidate: `NameLookup<StringId>` keys, numeric
string-token lookup and `StringId` query handles. Production structures and
flows are unchanged.

Required invariant: a primary table row is both the sole stored normalized name
and its typed identity slot. No parallel candidate string table, `StringId`, or
`string token -> typed ID` mapping may exist.

## Reintroduction guard

Root cause guarded: treating interned text identity and domain identity as two
candidate layers creates the second correspondence the hypothesis removes.

Single allowed candidate flow:

```text
raw &str -> normalized owned/borrowed text ->
  primary table position == typed ID
  OR alias text -> typed ID
```

Final review SHALL verify:

- both timed variants receive the same raw `&str` and normalize inside lookup;
- candidate types and timed closures do not mention `StringId`;
- `StringId` appears only in current-baseline/source-projection state and its
  lookup compares through the existing snapshot string table;
- the transient legacy-key projection map is absent from measured retained
  state;
- primary type lookup returns the found row ordinal as `TypeId`;
- callable/property name ordinals are family-specific and completed with owner;
- scoped primary lookup searches one completed-ID vector by
  `(owner, family_names[id.name])`, and a name present only under another owner
  remains missing;
- aliases allocate no identity and route to exactly one index;
- the candidate has no reverse map, parallel string-token table, fallback
  search, production snapshot/read/materialize/X1 diff or public export;
- duplicate and excluded-corpus counts are printed by the frozen run;
- the actual retained-byte accounting includes candidate-owned name bytes.

## Codebase-Design Review Record

### Pre-implementation pass — 2026-08-05 — PASS after revision

- Scope: one existing private feature-gated experiment module and frozen
  runner; production snapshot/read/materialize/X1 interfaces remain unchanged.
- Interface depth: the candidate exposes only construction, routed raw lookup
  and retained-byte observation. Family-specific identity layout is expressed
  through typed tokens/IDs behind the one generic lookup mechanism.
- Owners/locality: the type table owns both primary text and `TypeId`; each
  scoped family name table owns its name ordinals; completed-ID primary vectors
  own owner membership; alias indexes own search-only keys. No layer mirrors
  another layer's complete record shape.
- Required seam: legacy `HbkNameView` resolution is confined to one source
  projection boundary. It produces borrowed text for candidate control and
  parallel existing snapshot keys only for the current baseline; neither is an
  adapter DTO exported from the module. Candidate formation and timing cannot
  observe `StringId`.
- Finding resolved: a name table alone cannot prove owner membership. One
  completed-ID vector per scoped family is necessary and sufficient; it avoids
  both false positives and a second binary search while not repeating owner or
  primary text.
- Alternatives rejected: composing an ID after only a global name-table hit
  permits false positives; owner-local copied name tables repeat common member
  names; a hash membership set adds a second lookup and retained structure;
  making the baseline own copied strings would no longer measure current
  lookup storage; changing production views would broaden a test-only
  hypothesis.
- Reuse/deletion test: deleting the scoped completed-ID vector loses owner
  existence semantics; deleting any additional key pool, reverse map, owner
  field or entity-ID mirror loses no behavior, so none is allowed.

### Actual-diff pass — 2026-08-05 — PASS

- Scope remained one existing private module behind
  `all(test, feature = "snapshot-experiment")`; production snapshot, reader,
  materializer, X1, public API and schemas have no diff.
- Actual candidate state is exactly `Vec<Box<str>>` family names, an empty
  zero-capacity primary vector for `TypeId`, one completed-ID primary vector
  for each scoped family and `Box<str> -> completed ID` alias entries. Alias
  capacity equals the number of retained alias entries.
- Type primary lookup binary-searches the name table and constructs `TypeId`
  from the found row index. Scoped lookup performs one binary range search over
  completed IDs and dereferences only the family name ordinal; the absent-under-
  another-owner regression passes.
- Candidate source/canonical/query rows and timed lookup contain no `StringId`.
  `StringId` remains only in explicitly named legacy rows and the transient
  current-baseline projection; the text-to-existing-key map is local to corpus
  preparation and is dropped before measurement.
- Raw routing executes one selected primary or alias search with no fallback.
  Both lanes normalize the same borrowed raw query inside timing; pre-timing
  differential verification reports no semantic mismatch.
- No duplicate string/token-to-domain-ID mapping, entity-ID mirror, owner
  mirror, registry, cache, reader, parser, serializer, public re-export or
  production conversion was added. The owner-membership vector is the one
  planned exception required for scoped existence semantics.
- The final immutable-string/exact-capacity pass reduced the experimental
  retained and allocation cost without changing the approved shape. Frozen
  measurements and their repeat bounds are recorded in `measurements.md`.
- Architecture/README updates are not required: this change evaluates a
  private test-only representation and changes no crate responsibility,
  dependency direction, provider boundary or shipped contract.
- Independent actual-diff review approved the final correction and report with
  no remaining findings.

## Risks / Trade-offs

- [Ordinal IDs shift across different snapshots] -> Bound identity to the
  immutable snapshot and do not claim persistence across HBK versions.
- [Normalization allocation can dominate lookup] -> Include it symmetrically
  and report end-to-end latency.
- [First-character routing is a corpus convention] -> Restrict conclusions and
  publish excluded counts.
- [Candidate-owned text may cost more than legacy shared strings] -> Include
  string payload and allocations in retained/construction evidence.
- [Temporary duplicate dropping hides malformed sources] -> Count it, label it
  temporary and keep production formation unchanged.

## Migration Plan

1. Implement deterministic typed-table formation and raw routed lookup tests.
2. Add same-input current-versus-candidate frozen measurements.
3. Run focused/full verification and the isolated release benchmark.
