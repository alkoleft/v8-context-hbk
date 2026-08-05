## Context

The repository has durable experiment evidence and active OpenSpec changes, but
no neutral place for potentially valuable follow-up ideas that have not been
accepted as work. Keeping them only in experiment conclusions makes discovery
difficult; copying them into `tasks.md` would incorrectly make them active
scope. The first records are the ID-only hash-index and `string-interner`
opportunities measured on the frozen HBK corpus.

The affected stakeholders and owners are:

- the canonical `project-governance` capability owns workflow policy;
- `project-governance/IMPROVEMENT_OPPORTUNITIES.md` owns non-authoritative
  candidate memory;
- linked reports and commits own measurement evidence;
- an apply-ready `openspec/changes/<change>/` owns accepted scope, design, and
  all task status after promotion.

## Goals / Non-Goals

**Goals:**

- retain evidence-backed improvement opportunities under stable identifiers;
- cover performance and non-performance categories with the same small record;
- distinguish evidence maturity from implementation progress;
- make the promotion boundary into OpenSpec explicit and auditable;
- record the two interning/indexing candidates without authorizing adoption.

**Non-Goals:**

- create a roadmap, backlog, priority queue, or implementation checklist;
- assign people, dates, releases, or execution status to opportunities;
- adopt a hash index, interning crate, public identity, or X1 format change;
- copy experimental implementation into the main branch;
- bootstrap a second project-governance control plane beside OpenSpec.

## Decisions

### Use one Markdown discovery registry

Create `project-governance/IMPROVEMENT_OPPORTUNITIES.md`. Each entry has a
stable `IMP-NNNN` identifier, categories, one evidence disposition, affected
areas, opportunity, hypothesis, expected value, evidence, constraints and
trade-offs, promotion trigger, review trigger, OpenSpec relationship, and last
reviewed date.

The allowed evidence dispositions are `captured`, `needs-evidence`,
`validated-candidate`, `conditional-candidate`, `promoted`, `rejected`, and
`superseded`. They describe what is known about an idea, never whether someone
is implementing it. A promoted entry is retained as provenance and links to
the OpenSpec change that took ownership.

Alternatives considered:

- one YAML file per idea plus a generated index provides machine-readable
  structure, but adds schema, synchronization, and tooling before the third
  entry exists;
- storing observations only in nearby active or archived change artifacts
  avoids a new file, but provides no cross-change discovery surface and does
  not satisfy the requested general registry;
- placing mutable records in canonical specs would confuse tentative ideas
  with normative requirements.

### Keep policy and execution ownership in OpenSpec

The existing `project-governance` capability gains the normative registry
contract. The registry is supporting evidence: listing an entry does not
authorize implementation or override a canonical spec. Promotion requires a
separate apply-ready OpenSpec change; its `tasks.md` becomes the only execution
ledger.

The registry forbids checkboxes, priority/scheduling labels, assignees,
implementation steps, execution states such as `in-progress` or `done`, and
rules that select the next entry. This is a deliberate shallow seam: evidence
is read directly by humans and agents, with no adapter or compatibility layer.

### Preserve evidence and make staleness review event-driven

Entries cite their origin and reproducible evidence when it exists. A review
trigger names the event that can invalidate or advance the idea, such as a
profile, ownership-contract change, or replacement experiment. Rejected,
superseded, and promoted entries remain in the registry as decision provenance
instead of silently disappearing.

Calendar expiry is not required: it would create a maintenance schedule and
implicit work queue. Any attempt to promote an entry, and any material change
to its affected contract, triggers an evidence review.

## Risks / Trade-offs

- [The registry becomes a shadow backlog] -> prohibit task semantics in both
  the canonical contract and the document; verify the added file structurally.
- [Tentative evidence is mistaken for a requirement] -> state precedence in
  every entry boundary and require a separate apply-ready OpenSpec change.
- [Evidence becomes stale] -> require provenance, last-reviewed and an
  event-based review trigger; retain explicit terminal dispositions.
- [One Markdown file becomes unwieldy] -> keep the one-file design until real
  volume demonstrates a need for per-entry files or generated indexes.
- [Experimental branch links disappear] -> cite immutable commits as the
  durable evidence identity, with the branch/worktree only as convenience.

## Migration Plan

1. Add the canonical delta before adding the registry document.
2. Create the registry and seed `IMP-0001` and `IMP-0002` from experiment
   commit `cf90c0f` without importing experimental code.
3. Validate the OpenSpec change and structurally check that the registry has no
   active-task syntax.
4. Apply the documentation/governance patch bump, archive with spec
   synchronization, validate canonical state, and commit the scoped files.

Rollback removes the registry and reverts the added canonical governance
requirement. The immutable experiment commit remains the evidence source, so
no runtime or stored-data migration is needed.

## Verification

- `openspec validate establish-improvement-opportunity-registry --strict`
- a focused search rejects task checkboxes and task-like fields in the registry;
- the two seeded records have unique IDs, allowed dispositions, immutable
  evidence references, and explicit promotion/review triggers;
- `Cargo.toml` and workspace packages in `Cargo.lock` report the same patch
  version;
- after archive, `openspec validate --specs --strict` and
  `openspec validate --changes --strict` pass.

The version classification is patch because the change affects only
documentation and governance.

## Codebase-Design Review Record

### Pre-implementation pass — PASS

- **Reviewed scope:** the proposed canonical governance delta, one discovery
  registry, its two initial entries, version provenance, and archive flow.
- **Interfaces:** `IMP-NNNN` entry schema for discovery; OpenSpec change
  artifacts for promotion and execution.
- **Seams and adapters:** one direct evidence-to-proposal promotion seam; no
  adapter, generated index, parser, or compatibility layer.
- **Owners:** canonical policy in `openspec/specs/project-governance`; candidate
  evidence in the registry; measurements in immutable experiment evidence;
  accepted work and task state in OpenSpec changes.
- **Findings:** a standalone registry risks duplicate workflow ownership; YAML
  cards and a generated index would add premature machinery; storing ideas only
  inside individual changes fails cross-change discovery.
- **Resolutions:** make the registry explicitly non-authoritative, prohibit
  execution semantics, choose one Markdown file, and require apply-ready
  OpenSpec promotion.
- **Outcome:** `PASS`; ownership is singular and the seam is narrower than the
  rejected alternatives.

### Actual-diff pass — PASS

- **Reviewed scope:** the implemented registry and both records, active
  OpenSpec artifacts, synchronized workspace-version diff, validation gates,
  and intended archive boundary.
- **Interfaces:** the document implements the planned `IMP-NNNN` evidence
  schema; promotion still crosses only through a separate apply-ready OpenSpec
  change.
- **Seams and adapters:** no parser, generated index, adapter, compatibility
  facade, runtime dependency, or code seam was added. The direct
  registry-to-OpenSpec promotion seam is unchanged from the reviewed design.
- **Owners:** the registry contains evidence disposition only; canonical specs
  own requirements, change artifacts own accepted scope and design, and only
  change `tasks.md` owns execution state. The patch bump owns completion
  provenance without changing a shipped API.
- **Findings:** the first review correctly blocked completion while this record
  still said `PENDING`. Independent review found no medium issues, confirmed
  that the registry contains no shadow task ledger, and matched every quoted
  decision-critical measurement to experiment archive commit `cf90c0f`.
- **Resolutions:** replace the pending marker with this complete record; retain
  structural rejection of task-like fields; keep the ID-only index a validated
  hypothesis and `string-interner` conditional on sole ownership and an
  end-to-end comparison.
- **Outcome:** `PASS`; the actual diff matches the approved structure, has one
  owner per concern, and leaves no blocking design finding.

## Open Questions

None. A future change may introduce machine-readable cards only after registry
volume or automation provides concrete evidence for that complexity.
