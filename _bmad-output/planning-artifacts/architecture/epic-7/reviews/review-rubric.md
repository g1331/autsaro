# R5 requirements and spine rubric review

Independent reviewer result; no build, test, linter or formatter was run by the reviewer. Parent persisted the reported findings because the reviewer filesystem write was unavailable. The reviewed PRD, spine and UX preserve selected R5 scope, diverse acceptance, inherited OS/profile contracts and the local deployment envelope. Editorial structure/prose supplied no additional substance-preserving changes.

## Reported findings

1. AD-8 conflated object navigation with project replacement. Applying a draft is not saving it; project replacement/reopen/handoff/exit must return-and-save, explicitly discard or cancel. Resolution: AD-8 now requires full draft apply, preview and confirmed save before pending replacement; failure cannot continue.
2. AD-9 did not reconcile mutable user application with strict sealed output consumers. Existing output.rs rejects changed listed or unlisted files; merely excluding a file from the generator write set is insufficient. Resolution: declared live application inputs remain outside sealed output; immutable snapshots and an ownership ledger enter the strict files.list/files.sha256 closure. Preview/build/handoff/offline consumers migrate together under a new v2 format; old formats remain strict.
3. AD-3/5 did not distinguish the current capability token from the reply request echo. Current useWorkbench sends capabilities.fingerprint; Reply.inputFingerprint echoes the old request even after revision publication. Resolution: projection/ChangeSet.inputFingerprint is current capabilities.fingerprint, while Reply.inputFingerprint is correlation only.

## Disposition

Parent incorporated all three findings in AD-3/5/8/9 and consistency/ownership contracts before finalization. This records the independent initial review and parent resolutions, not a second reviewer approval or runtime verification.

## 2026-10-03 native-rule amendment — independent review and targeted closure

Fresh independent rubric review initially returned CONCERNS, confidence 0.92. It confirmed retention of all 12 CFG subrequirements, 5 NFRs, 8 UX requirements and 11 Story IDs/order, general editing/batch/reference/structure behavior, source safety, unknown-content preservation, native rule coverage/trust and developer-oracle versus user-install verification separation. No editorial change was needed.

Two real contract gaps were reported and fixed:

1. Story 7.6 required archive-free package handoff before 7.10 introduced v2, while old v1 still requires its original external identities. Resolution: 7.6 proves source configuration, reopen, relocation and pure-source generation without official archives; new v2 handoff belongs to 7.10/7.11, and strict resource-backed v1 compatibility remains separate.
2. Project-local accepted extension identities had no persistence contract. Resolution: the project manifest stores exact acceptedExtensionDefinitions; 7.2 owns verified immutable digest-keyed catalog cache installation, and 7.6 atomically persists the project acceptance set with source/member metadata. Raw ARXML acceptance is session-local, cache presence is not acceptance, and reopen never selects machine-global catalogs or substitutes versions. Missing/corrupt definitions only limit actual consumers.

The same independent reviewer inspected the applied correction clauses in spine AD-7/Catalog persistence and Story 7.2/7.6, returning targeted PASS with no remaining blocker, confidence 0.96. Parent also aligned PRD CFG-1.3, Story 7.9 and EXPERIENCE settings to the same acceptance/cache/dirty metadata mechanism. This is evidence of the initial full review plus targeted correction confirmation, not a repeated full-artifact or runtime review.

Reviewer was read-only; parent persisted these returned findings and confirmations. No builds, lint, tests, formatters or implementation checks were run by this reviewer.

