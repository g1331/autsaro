# Changelog

[简体中文](CHANGELOG.zh-CN.md) · **English**

## Unreleased

- Make reader-facing root documents English by default, retaining complete Chinese versions with the `.zh-CN.md` suffix and updating reciprocal language links.
- License original project code and documentation under Apache-2.0; add package metadata and LICENSE/NOTICE files to delivered projects. Third-party content retains its original licenses.
- Use npm, Cargo, and uv directly for development, removing duplicate task orchestration and separating tooling source, tests, and optional skill maintenance.
- Support Python 3.12+ for development and 3.11+ for offline tools without requiring a specific patch version; use Vitest for frontend tests.
- Separate project, editing, delivery, and settings logic while preserving IPC and atomic-commit boundaries. Draft changes invalidate previous batch previews.
- Assign new trusted identities when resource paths or tools change. Existing sealed packages must still match their original producer and are not rewritten in place.
- Separate basic tests, official comparisons, and native execution tests; add frontend behavioral regression coverage.
- Add a contributor guide, basic CI, dedicated acceptance entrypoints, and asset-maintenance documentation.
- Complete Git line-ending rules needed to preserve runtime byte identity.
- Plan projects with a single application component on the multi-component mainline; a composition with only service components is rejected with `APPLICATION_COMPONENT_MISSING`.
- Mainline Dcm refuses ReadDataByIdentifier requests with more DIDs than the configured `DcmDspMaxDidToRead` with NRC 0x13 (SWS_Dcm_01335). `Dcm_ConfigType` gains `max_did_to_read` (0 when not configured) and plan descriptions record `maxDidToRead`.
- Mainline CanTp reports `CANTP_E_RX_COM`/`CANTP_E_TX_COM` runtime errors on timeouts (SWS_CanTp_00229). The generated host target receives them through a Det runtime callout and reports them as HostBatch transport failures.
- Mainline projects with one application component, one 32-bit CAN receive and transmit signal and an application DID deliver `verification/inputs.json`, so `ecu-tool verify` checks the reference echo behaviour.
- Mainline plans reject CanIf PDU handles above 255 with `CANIF_HANDLE_WIDTH` (the delivered `PduIdType` is `uint8_t`) instead of `CAN_ID_CONFLICT`, and report a missing selected BSW entry at the module reference that names it instead of an unrelated object.

Early development results remain in Git history and the corresponding implementation artifacts. This changelog does not restate historical releases or acceptance results.
