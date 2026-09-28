# Epic 4 obligation and oracle baseline

`obligations.json` traces current R24-11 OS requirements, OSEK services and
capacity dimensions, and the selected reference integration clauses to their
configuration, producer, responsible story, test, and future evidence location.
`os-source-index.json` records source coordinates and paragraph hashes without
redistributing normative prose. A reference fixture count cannot replace the
capacity required for any supported conformance class.

`sources.json` pins the external documents, original FreeRTOS archive and source
closure, production patches, separate research material, and compiler identity.
Official source documents remain external. The future generated binary import
and runtime notice audit is an explicit packaging gate in stories 4.20/4.21.

The independent vectors in `core/tests/fixtures/epic4_oracles/` declare inputs,
expected observations, timing conditions, and their source. They are reviewed
against the original reference configuration and integration contract, rather
than calculated by the generated implementation. Their future execution remains
`not_run`; a baseline check does not demonstrate target behavior or SC1 compliance.

Run `cargo test --manifest-path core/Cargo.toml
epic4_obligation_and_oracle_baseline -- --exact --nocapture` for the registered
entry. The underlying verifier accepts `--osek-pdf`, `--kernel-archive`, and
`--compiler` to locate the pinned external inputs. Missing inputs or identity
mismatches fail the entry; no source is downloaded during verification. PyMuPDF
is required for checking the current normative paragraph index.

Applicability decisions require independent review in addition to structural
verification. Later stories must attach actual behavioral evidence to applicable
rows and justify conditional exclusions before the final Epic 4 exit can pass.

`reviewed-baseline.json` seals the reviewed trace, applicability, case inventories,
complete oracle contents, and original reference inputs. It is an acceptance
artifact; no build, generator, or target implementation rewrites it. Changes need
source-backed independent review before updating the seal. Structural mutation
tests protect against omitted requirements and altered independent expectations.
