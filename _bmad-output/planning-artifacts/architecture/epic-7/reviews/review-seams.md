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

## 2026-10-03 native-rule amendment — independent PASS

Fresh independent seam review returned no concrete incompatible compliant pair or blocker, confidence 0.94. Parent persisted this result because the reviewer was read-only; the earlier external-resource resolutions above remain historical and do not override the new native-rule contract.

Pairs checked against the current spine, selected PRD, Epic 7 and prospective EXPERIENCE:

- Rule packager / runtime verifier: the same trusted RuleSetIdentity binds implementation provenance, coverage and immutable metadata.
- Project catalog / generation and import catalog: all project-accepted definitions and the actual consumer closure are distinguished; consumers rebuild opaque fingerprints rather than trust copied tokens.
- Prepare / apply, save and generation confirmation: rule, definition, source and draft changes invalidate confirmations.
- Validator / editor, save and generator: supported native schema rules actually execute and errors reject; unsupported and not_run are distinct; safe unchanged old-definition repairs and unknown-content preservation do not authorize unsupported generation.
- Extension exporter / importer: catalog acquisition, inventory/member hashes, explicit acceptance and isolation are fixed.
- v2 producer / importer and legacy dispatch: exact native identity, required extension identities and complete source/rerender/seal closure bind new-directory migration; v1 retains original external identities and separate strict checks, without hash substitution.
- Ownership writer / downstream consumers: existing Session/reducer, owned staging, immutable application snapshots and strict seals remain shared.

Reviewer ran no builds, lint, tests or formatters and changed no planning/source files. PASS is a planning seam judgment only.

