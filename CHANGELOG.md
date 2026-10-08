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

Early development results remain in Git history and the corresponding implementation artifacts. This changelog does not restate historical releases or acceptance results.
