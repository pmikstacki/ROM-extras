# Quality gates

Use the [documentation writing rules](writing.md) for technical prose.

These are ROM's adopted delivery criteria. When the Rust and database components exist, their gates become executable. This document does not claim they pass today.

## Module boundaries

Keep `lib.rs` and `mod.rs` as module facades. These files contain module declarations, imports, exports and definitions required across modules.
Put implementation logic in named modules with a clear responsibility. Put unit tests in separate test modules. Keep public paths stable through exports.
Rust requires procedural macro entry points at the crate root. These functions may delegate to an expansion module; parsing and generation stay there.
Remove duplicate behavior through shared contracts and helpers. Do not create a generic abstraction when the operations have different guarantees.
After a structural change, run the affected tests, downstream compile fixtures and the full local verifier before integration.

- Run `./scripts/check` locally before sharing changes. GitHub hosts code only; Actions is disabled. Behavior changes carry concrete success and failure scenarios.
- The first Rust crate adds formatting, Clippy with warnings denied, workspace tests and doctests. It also adds documentation with warnings denied and a check of the declared minimum Rust version.
- The domain core forbids unsafe Rust and denies ignored must-use results. Any future unsafe adapter requires separate justification and documented invariants.
- An external example must implement a custom field using only public interfaces.
- Authoring reviews cover a human's path through declaration, custom behavior, live observation and failure diagnosis. Readability, discoverability and debugging effort matter alongside boilerplate reduction. Agent-authored examples alone do not establish human usability.
- Assess complexity exposed to application authors separately from complexity inside ROM. Framework-owned helpers and generated bindings should absorb difficult generic and infrastructure work when that improves the public interface.
- Derives and fluent interfaces share one semantic contract. Compile-failure fixtures test deliberate diagnostics, source locations and supported edge cases. Generated behavior is inspectable, and a manual implementation route is documented.
- Every supported durable database adapter passes the same real-database tests for atomic state/event writes, rollback, stale revisions and restart recovery.
- Mutation tests distinguish missing, null, false, zero and empty values. Rejected actions produce no state change or success event.
- Reactive tests inject interruption before and after commit and acknowledgement, exercise duplicates and retries, and verify bounded work and documented ordering.
- Dependency adoption checks advisories, licenses, supported Rust versions and required features. Test the core without transport integrations.
- Releases require compatibility review, migration notes and a packaged-consumer smoke test.

The [Beskid compiler review](research/beskid-compiler-lessons.md) adds these concrete acceptance criteria:

- Derived and manual declarations pass the same registration gate. Record its invariants. Freeze accepted definitions. Current values and permissions still need runtime checks.
- Descriptor/codec conformance covers public names, missing/null/default behavior and custom fields. Compare actual encoded/decoded values, not only descriptor equality.
- Maintain a supported-capability matrix with negative cases. Distinguish unsupported capability, invalid declaration, ordinary request rejection and internal invariant failure in structured errors.
- Negative fixtures fail for their intended reason and retain source/path context. Test downstream consumers independently of workspace feature unification.
- Verification records identify the executed source, relevant dirty changes, lockfile, compiler, command and result. Source inspection, reported tests and locally reproduced tests are separate evidence categories.

The [Beskid baseline audit](research/beskid-quality-baseline.md) explains the evidence behind these criteria. This is a stronger target in specific areas, not a claim that an unimplemented library already has higher code quality.
