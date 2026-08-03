## Decision

`syntax-helper-search` is the single HBK implementation owner for the common
semantic roles because it owns `HbkFactSnapshot`, `HbkFactReadHandle`, the
public storage-neutral `Hbk*View` family and both owned and X1 source-backed
iterators. Only this crate receives the direct neutral dependency.

The existing views remain the public deep module. The callable, parameter and
property contracts require `&str`, while current owned views contain a record
reference whose names are `StringId`s. A separate read-bound semantic adapter
family would add a second shallow public surface. Instead, only the relevant
owned view variants and their child iterators retain the already-borrowed
`HbkFactSnapshot`; relevant mapped variants continue to resolve through their
existing X1 handle. Existing source accessors and call sites remain valid.

The trait implementation matrix is exact:

| Existing/local HBK value | Common role | Classification/evidence |
| --- | --- | --- |
| `HbkCallableView` | `CallableView` | platform origin; `Some(owner)` is platform-type ownership, `None` is global-context ownership |
| `HbkSignatureView` | `SignatureView` | existing parameter and signature result-type iterators |
| `HbkParameterView` | `ParameterView` | existing name/type refs; bool requiredness; passing is source-unspecified |
| `HbkPropertyView` | `PropertyView` | one filtered view over existing property/enum-value members or BSL global-property facts |
| `HbkPlatformTypeView` | `TypeDeclarationView` | `Name<'a> = HbkNameView<'a>`; `Owner<'a> = StringId`; `owner()` returns the existing stored `id()`; platform-type ownership |

Callable kind mapping uses only source-proved facts: `Method` and
`GlobalMethod` map to common `Method`, `Constructor` to `Constructor`, `Event`
to `Event`, and `LanguageFunction` to `Function`. No HBK record proves BSL
procedure semantics. Member mapping accepts only `Property` and `EnumValue`;
`Method` and `Event` return no property role. Global-fact mapping accepts only
`HbkGlobalFactKind::Property` in `HbkLanguageDomain::Bsl`; methods and other
language domains return no property role.

Declared type values remain `HbkTypeRefView`. The common leaf performs no type
comparison, availability filtering, selection or overload ranking. The
existing HBK indexes and IDs remain generation-local source state.

## Task-local plan for task 1.1

1. Add the workspace path dependency and consume it only from
   `syntax-helper-search`.
2. Deepen the relevant existing view/iterator owned variants with the existing
   snapshot borrow; add private/inherent borrowed-name resolution shared by the
   direct trait impls. Preserve current public accessors and storage order.
3. Add the five role implementations in one focused snapshot-owned module and
   the single `HbkPropertyView` filtered view over either an existing member or
   BSL global fact. Keep its representation private so method/event/non-BSL
   values cannot construct the role. Do not add another callable, signature,
   parameter, property or type record.
4. Add owner-local behavior tests that run the same role observations against
   owned and canonical X1 snapshots, exercise neutral argument-count matching,
   and prove method/event/non-BSL property rejection. Assert the exact
   `HbkNameView`/stored-`StringId` type-declaration contract. Add a source guard
   for the exact dependency/impl/wrapper matrix and prohibited copies, indexes,
   allocating getters and neutral re-exports.
5. Update `spec/implementation/components.md`, reconcile the actual diff,
   perform fresh correctness and codebase-design review, run format, strict
   Clippy, focused and workspace tests plus strict OpenSpec validation, bump
   the completed internal change from `0.2.5` to `0.2.6`, archive it with spec
   synchronization and commit only task-scoped HBK files. Downstream lock,
   ledger, task and architecture reconciliation is a separate post-HBK
   integration step owned by the analyzer change, not task 1.1 acceptance.

Exact verification commands before completion:

```text
cargo fmt --all -- --check
cargo clippy -p syntax-helper-search --all-targets --all-features -- -D warnings
cargo test -p syntax-helper-search semantic_roles
cargo test -p syntax-helper-search snapshot::x1_format::tests::semantic_roles_have_owned_and_mapped_parity -- --exact
cargo test --workspace
openspec validate expose-borrowed-semantic-role-capabilities --strict
git diff --check
```

The structural test and final source review additionally require the neutral
dependency declaration only in the workspace plus `syntax-helper-search`
manifests; the exact direct impl matrix; exactly one public
`HbkPropertyView`; no `HbkSemantic*`/common entity record, copied semantic
collection, allocating `String`/`Vec`/boxed getter, second semantic index or
registry, manual argument-count implementation, cache/schema/serializer
mapping, analyzer import or `pub use` of `v8_context_semantic_entities`.
After task completion, archive with spec synchronization, validate canonical
OpenSpec state strictly and inspect the staged diff before commit.

### Structure impact

- Searched owners and consumers: `HbkFactSnapshot`, `HbkFactReadHandle`,
  `HbkCallableView`, `HbkSignatureView`, `HbkParameterView`,
  `HbkTypeMemberView`, `HbkPlatformTypeView`, their H0/X1 inner views and
  iterators, `HbkBslContextCatalog`, resolver projection helpers, analyzer
  `context-provider`, exporter DTOs, tests/fixtures, manifests, cache codecs,
  `HbkGlobalFactView`, schemas/generators and CLI. Searches covered `id`,
  `owner`, `domain`, `kind`, `name`,
  `signatures`, `parameters`, `required`, `type_refs`, `return_type_refs`,
  `availability_contexts`, argument counts, `StringId` resolution and existing
  public re-exports.
- Real consumers remain the existing catalog/resolver/analyzer paths; the new
  roles add a generic algorithm input and do not replace candidate acquisition,
  context selection or serialized output.
- Reused: the snapshot dictionary and read handle, typed source IDs, existing
  records/views, source-backed H0/X1 iterators, source order, X1 validation,
  availability evidence and neutral traits/arity behavior.
- Added: one direct dependency edge, direct trait impls, one kind/domain-checked
  copy view whose private closed representation contains either one existing
  `HbkTypeMemberView` or one BSL `HbkGlobalFactView`, private borrowed-name
  access, and the minimum snapshot borrow in relevant owned view/iterator
  variants. This changes no record, cache, schema or serialized shape.
- Not added: copied name/owner/signature/parameter/type/availability fields,
  source locator, DTO/read model, conversion chain, parser, normalizer, loader,
  serializer, cache key/row, registry, index, interner, alternate catalog,
  dynamic dispatch, boxed/allocating iterator, analyzer dependency, neutral
  facade re-export or manual arity algorithm.
- Deleted: no production behavior is currently duplicated by the shared arity
  owner; the repository search found no HBK manual argument-count predicate to
  delete. No existing record/index is deleted because it is the accepted sole
  provider storage/lookup owner.
- Checked untouched boundaries: extraction/model/export DTOs, SQL/search
  storage, X1 physical format and validation, context-resolver response DTOs,
  CLI/provider JSON, frontend (absent), schemas/generators and analyzer
  selection/type inference.

### Reintroduction guard

The root cause being prevented is consumer pressure to pair provider IDs with
copied names/signatures in a new common HBK projection. The single allowed
owner/flow is
`HbkFactSnapshot -> existing Hbk*View (or filtered HbkPropertyView) -> neutral
trait/algorithm -> operation-local caller outcome`.

Behavior and structural verification must fail if a future change adds an
owned `HbkSemantic*`/common entity record, another callable/signature/parameter
or type collection, an allocating semantic getter, another lookup index or
registry, a second property wrapper or parallel global-property seam, a manual
argument-count predicate, a cache/schema/serializer mapping, or `pub use` of
the neutral crate. Owned/X1
parity and pointer/borrow checks protect the direct dictionary-backed flow even
when a duplicate could preserve output.

## Pre-implementation codebase-design review

**PASS (2026-08-03).** The pass reviewed task 1.1, `snapshot/views.rs`,
`snapshot/read.rs`, the H0/X1 child iterators, the planned role module and the
single filtered property seam.

- `HbkFactSnapshot`/`HbkFactReadHandle` plus `Hbk*View` remain the deep module:
  callers receive storage-neutral views while dictionary ownership, H0/X1
  representation and child traversal stay hidden. Adding the existing snapshot
  borrow only to relevant owned variants closes a real missing capability and
  is preferable to a parallel read-bound adapter family.
- One focused `semantic_roles` module is cohesive rather than pass-through: it
  owns the provider-to-neutral kind/owner/evidence mapping, the common trait
  implementations and the invariant-checked property view. It owns no storage,
  lookup or conversion pipeline.
- `HbkPropertyView` is a justified closed seam because type-member and BSL
  global-property records have distinct source representations but one proved
  property contract. Its private representation prevents invalid method,
  event and non-BSL construction without copying their fields.
- Direct implementations on callable/signature/parameter/platform-type views
  maximize leverage and locality. Associated source values and existing
  iterators avoid DTOs, boxed dispatch and shallow getters returning owned
  collections.
- No additional facade, adapter, generic extension point or module split is
  justified. The planned module boundaries match the approved Structure impact
  and Reintroduction guard.

No actionable design finding remains; production implementation may start.

## Actual-diff codebase-design review

**PASS (2026-08-03).** The pass reviewed every production, test, manifest,
lockfile and architecture change against the approved plan.

- `semantic_roles.rs` is cohesive: it owns the five direct role
  implementations, exhaustive callable mapping and the single invariant-
  checked property seam. It owns no source acquisition, storage, lookup,
  availability, type equality or consumer answer.
- `snapshot_storage_view!` expresses one distinct internal invariant: only
  owned views that must resolve role names or propagate that resolver to child
  parameters retain the existing snapshot borrow. Mapped views keep their
  existing X1 handle. Platform types and unrelated query/enum views remain on
  the original storage-only macro.
- `HbkPropertyView` plus its private two-variant representation is the one
  planned public seam, not a second record family. Both variants hold an
  existing copy view and no semantic field. Its only local facade export is
  intentional; the neutral dependency is not re-exported.
- Existing signature, parameter and type-reference iterators remain the sole
  child traversal. No allocating getter, boxed dispatch, source locator,
  registry, index, cache/schema mapping, serializer or conversion chain was
  added.
- The owned/X1 parity test exercises dictionary-backed names, overload and
  parameter order, common arity, required/passing evidence, declared types,
  member/global properties, invalid-role rejection and the exact platform-type
  identity. The structural test rejects the named duplicate/re-export paths.

Actual Structure impact reconciliation: the dependency edge, direct impls,
private `snapshot_storage_view!` support, borrowed name methods, one public
`HbkPropertyView` with one private representation enum, one intentional local
facade export and tests are all listed by the approved plan. No unlisted
semantic structure, reusable data path, mapping, conversion or public surface
appears in the diff. No actionable design finding remains.

## Final review and verification

**PASS (2026-08-03).** A fresh correctness review found one ineffective test
assertion: the owned callable borrow guard compared the role name pointer with
itself. The guard now compares that pointer with the callable primary-name
entry in the owning snapshot dictionary. Focused verification passed after the
correction, and the reviewer returned `NO FINDINGS` on the corrected diff.

The completed slice passed:

- `cargo fmt --all -- --check`
- `cargo clippy -p syntax-helper-search --all-targets --all-features -- -D warnings`
- `cargo test -p syntax-helper-search semantic_roles -- --nocapture`
- `cargo test -p syntax-helper-search snapshot::x1_format::tests::semantic_roles_have_owned_and_mapped_parity -- --exact`
- `cargo test -p syntax-helper-search --test semantic_role_boundary`
- `cargo test -p syntax-helper-search`
- `cargo test -p context-resolver-search`
- `CARGO_INCREMENTAL=0 cargo test --workspace`
- `openspec validate expose-borrowed-semantic-role-capabilities --strict`
- `git diff --check`

The final source search found the neutral dependency only in the root workspace
and `syntax-helper-search` manifests; no neutral `pub use`, `HbkSemantic*`,
common snapshot/catalog/index/registry or manual argument-count implementation
was present. The implementation therefore satisfies the approved Structure
impact and Reintroduction guard without transferring HBK storage or lookup
ownership.
