## Context

`QueryTableSnapshotSource` already maps provider-owned snapshot records to the
public resolver facts used by in-process consumers. It offers owner-scoped
point queries for a named field or parameter, while the underlying immutable
snapshot already owns ordered indexes for every field and parameter of a table.
The only public enumeration route currently returns all SDBL facts, forcing a
consumer to over-read unrelated tables or to bypass the adapter.

## Goals / Non-Goals

**Goals:**

- Expose owner-scoped, deterministic field and parameter enumeration at the
  existing `QueryTableSnapshotSource` boundary.
- Preserve existing fact mapping, source activation, owner validation and
  response semantics.
- Reuse the provider-owned snapshot index without allocating a cache, index or
  intermediate table-member model.

**Non-Goals:**

- Changing `ContextSource`, the resolver DTO model, snapshot storage, SQLite
  schema, binary cache, query parsing or analyzer behavior.
- Adding aliases, generic interpretation or cross-table selection semantics.
- Replacing existing point methods or making a global-context scan a fallback.

## Decisions

1. Add two inherent methods to `QueryTableSnapshotSource`, one for fields and
   one for parameters, accepting the existing public `FactId` and
   `ResolveContext` and returning `ResolveResponse<ContextFact>`.
   This keeps the source-specific query surface narrow; extending the generic
   `ContextSource` trait would impose a query-table concept on unrelated
   sources.

2. Reuse `HbkFactReadHandle::query_fields` and `query_parameters` only after
   the same source/domain/kind/active-owner validation as the point methods.
   The snapshot owns both deterministic order and physical owner index, so no
   consumer-side scan or mirror is necessary.

   Extract that repeated validation and exact table-id preparation into one
   private adapter helper, then make both existing point methods and the two
   enumeration methods use it. This removes an existing duplicate path without
   changing the public interface or creating a reusable model.

3. Return `NotFound` for inactive, mismatched or unknown table identities and
   `Ok` with an empty fact list for a known table with no members. This matches
   the resolver distinction between normal absence and a valid empty owned
   collection, without adding a new error or unknown type.

4. Keep field/parameter mapping in the existing adapter functions. The new
   methods enumerate mapped provider facts directly; they do not introduce a
   field-for-field public wrapper or conversion chain.

## Risks / Trade-offs

- [Caller confuses a field/parameter id with a table id] → validate source,
  domain and `QueryTable` kind before touching the snapshot index.
- [Future storage order changes] → contract tests assert deterministic
  owner-scoped order and point/enumeration consistency at the public boundary.
- [A consumer scans global context anyway] → document the owner-scoped API as
  the required route and retain the direct-index test as the ownership guard.

## Architecture impact

None. This deepens the existing `QueryTableSnapshotSource` interface at its
current resolver seam; crate responsibilities, dependency direction, storage,
cache behavior and deployment assumptions do not change.
