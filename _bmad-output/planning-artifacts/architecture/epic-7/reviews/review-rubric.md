# R5 requirements and spine rubric review

Independent reviewer result; no build, test, linter or formatter was run by the reviewer. Parent persisted the reported findings because the reviewer filesystem write was unavailable. The reviewed PRD, spine and UX preserve selected R5 scope, diverse acceptance, inherited OS/profile contracts and the local deployment envelope. Editorial structure/prose supplied no additional substance-preserving changes.

## Reported findings

1. AD-8 conflated object navigation with project replacement. Applying a draft is not saving it; project replacement/reopen/handoff/exit must return-and-save, explicitly discard or cancel. Resolution: AD-8 now requires full draft apply, preview and confirmed save before pending replacement; failure cannot continue.
2. AD-9 did not reconcile mutable user application with strict sealed output consumers. Existing output.rs rejects changed listed or unlisted files; merely excluding a file from the generator write set is insufficient. Resolution: declared live application inputs remain outside sealed output; immutable snapshots and an ownership ledger enter the strict files.list/files.sha256 closure. Preview/build/handoff/offline consumers migrate together under a new v2 format; old formats remain strict.
3. AD-3/5 did not distinguish the current capability token from the reply request echo. Current useWorkbench sends capabilities.fingerprint; Reply.inputFingerprint echoes the old request even after revision publication. Resolution: projection/ChangeSet.inputFingerprint is current capabilities.fingerprint, while Reply.inputFingerprint is correlation only.

## Disposition

Parent incorporated all three findings in AD-3/5/8/9 and consistency/ownership contracts before finalization. This records the independent initial review and parent resolutions, not a second reviewer approval or runtime verification.
