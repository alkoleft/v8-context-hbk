# Project Rules for Agents

These rules apply to the whole repository.

## Project Context

`v8-context-hbk` is a Rust workspace for reading 1C `*.hbk` help books and extracting structured platform documentation/context from Syntax Assistant books.

The project is currently an independently testable component with provisional contracts. It may later become an HBK-backed source for `/home/alko/develop/open-source/v8-context/`, but do not couple it to unfinished downstream contracts before the extraction model is validated on real HBK data.

Use these files as the main sources of project truth:

- `README.md`
- `docs/hbk-components-requirements-plan.md`
- `spec/IMPLEMENTATION_TODO.md`

## Context Boundaries

Keep context boundaries explicit and narrow.

- Separate HBK container reading, documentation navigation, Syntax Assistant extraction, domain modeling, export adapters and CLI/UI concerns.
- Validation belongs at boundaries: file/container input, external command input, parsing boundaries, serialization/export boundaries and public API boundaries.
- Do not scatter defensive validation through internal flows when data has already crossed a checked boundary and is represented by domain types.
- Prefer typed domain structures over repeated primitive tuples, stringly typed markers or duplicated ad hoc checks.
- Preserve provenance where it matters for diagnostics: HBK file path, entity name, TOC path, HTML path and page title.
- Keep runtime 1C introspection out of this repository unless the project plan explicitly changes. This project extracts documentation from HBK sources.
- Legacy-shaped DTOs or exports are adapters for concrete consumers, not constraints on the internal model.

## Design Principles

Optimize for simple, direct, maintainable code.

- KISS: choose the smallest design that expresses the current behavior clearly.
- YAGNI: do not add compatibility layers, extension points, caches, generic pipelines or configuration knobs until there is a concrete requirement.
- DRY: extract shared behavior when duplication starts encoding the same rule in more than one place. Do not create premature abstractions for merely similar code.
- Big Design Up Front: do enough upfront design to define context boundaries, data contracts, invariants and failure modes before implementation. Do not turn this into speculative architecture for unvalidated future integrations.
- Occam's Razor: when two designs satisfy the same contract, choose the one with fewer moving parts.
- Law of Demeter: keep modules talking through their immediate public interfaces. Avoid reaching through nested internals or binding one context to another context's representation details.
- SRP: each module/type should have one reason to change.
- ISP: expose small interfaces focused on real consumers. Do not force CLI, extractor, exporter and library users through one broad facade.
- Minimize boilerplate. Prefer language features, small helpers and clear domain types over repetitive templates.

## Testing Rules

Test behavior, not implementation.

- Treat the unit of testing as a unit of behavior: a public API contract, parser outcome, CLI result, export shape, error contract or documented requirement.
- Do not test private implementation details, helper call order, internal struct layout or incidental decomposition.
- Favor externally observable contracts: returned data, errors, diagnostics, serialized output and CLI behavior.
- Use small deterministic fixtures for parser behavior. Fixtures should represent real HBK/Syntax Assistant structures, not invented cases that only satisfy current code.
- Regression tests should describe the user-visible or contract-visible behavior being protected.
- When refactoring without behavior change, existing behavior tests should remain valid.
- Add broader tests when a change crosses module boundaries or changes a public contract; keep tests focused for local implementation changes.

## Implementation Discipline

- Follow the active implementation plan before adding new scope.
- Keep public contracts provisional unless the plan or ADRs explicitly stabilize them.
- Prefer Rust-native models and algorithms over reproducing Java/Kotlin reference APIs.
- Use typed errors instead of panics for recoverable input, parsing and export failures.
- Keep generated outputs, acceptance artifacts and experimental data out of source files unless the plan asks for durable artifacts.
- Do not introduce unrelated refactors while implementing a task.
