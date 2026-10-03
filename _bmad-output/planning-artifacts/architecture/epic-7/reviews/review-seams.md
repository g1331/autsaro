# R5 spine seam review

Independent reviewer result, confidence 0.90: five real cross-story contract gaps. Parent persisted the returned findings because reviewer filesystem writes were unavailable. No runtime, tests, lint or deliverable changes were performed by this reviewer.

## Reported incompatible pairs and resolutions

1. **Edit wire and new-target addressing.** Frontend could use local create keys/internal op tags while backend expects nested payload/external tags or actual IDs not yet allocated. Resolution: shared internal `op` union, ObjectRef existing/created, FieldRef existing/new, absent/explicit value states and allocated-ID mappings are bound in Change wire; prospective-graph validation permits required descendants and new-reference targets within one batch.
2. **Change preview confirmation.** Frontend could display only local draft changes while backend expands inbound rewrites; draft changes do not advance Workspace fingerprint. Resolution: nonmutating prepareChange emits actual impacts and changeRevision bound to normalized batch, input and catalog; applyChange confirms exactly it, draft changes invalidate confirmation.
3. **Definition fingerprint.** Frontend could send MOD digest while backend expects the assembled builtin/MOD/user catalog identity. Resolution: ProjectProjection exposes an opaque backend definitionFingerprint for all catalog states, including builtin-only/no catalog; clients echo it, never calculate it.
4. **Validation scopes.** Frontend and backend could disagree whether unchanged invalid fields block repair/save; inherited save rejects all errors including unsupported generation. Resolution: source-safety/schema/definition/target-generation diagnostics and before/after stable constraint witnesses are bound; unchanged old definition violations remain visible, new/worsened ones reject; actual Schema errors reject saves, missing Schema stays not_run, generation retains full finite validation.
5. **Ownership manifest.** Preview could classify owners only in records while regeneration expects a sidecar; old closure rejects edited or unlisted user applications. Resolution: workbench-ownership.json v1 is shared across consumers, live application inputs separate from immutable output snapshots, strict seals remain, new v2 handoff carries original sources and membership, old wire identities are not weakened. Ledger excludes itself/seal metadata from payload records to prevent self-digest cycles; owners remain checked against producer policy, not trusted permissions.

## Disposition

Parent applied these binding rules before committing the final architecture. Source-member atomicity, numeric lexeme preservation, unsupported-variant readonly behavior and finite generator boundaries were already binding and required no additional changes. This is the original independent review plus parent closure, not an independent re-review of the final text.
