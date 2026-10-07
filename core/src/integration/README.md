# Epic 4 standard input plan

`build_plan` is the all-or-nothing W2 constructor for `epic4-win64-sr-cs-v1`.
It receives original `InputSource` bytes, legally supplied pinned R24-11
XSD/MOD archives, and the current source-backed `RuntimeCatalog`. The result
has private fields, no unchecked constructor, no mutable accessor and no
`Deserialize` implementation. Consumers borrow `description()` and `sources()`.

`Workspace::integration_plan` uses the same constructor for the current
in-memory inputs. It rejects external edits to saved sources and derives
portable file identities below their common input root. This path preserves
the sources and operates independently of the legacy host-v1 profile checker.
The plan's raw-byte identities include unsaved edits. Saving those edits and
generating an ECU are separate operations.

The preflight `InputInspection` checks XSD validity and typed references.
Before generation, the final constructor also checks the selected
component/type/service relationships, communication and
canonical PDU links, task/alarm scheduling, routing, explicit target configuration
policies, and unique symbols/handles. Original bytes remain the authority for
roundtrip edits; the semantic graph is never serialized back into XML.

The graph uses full AUTOSAR paths. A source may contain retained unrelated
objects, including duplicate short names at distinct paths. Generated handles
are allocated deterministically in their domains. Configured CanIf/PduR IDs keep
their direction/API namespaces; equal Rx and Tx numbers do not merge objects.
Physical CAN identifiers must remain unique in the selected network.

`runtime/contracts/bsw-v1.json` records 17 current host BSW entry signatures and
their actual source/header identities. It is compiled into this workbench and
the supplied runtime tree must match. Verify it with
`uv run --locked python -m autosar_tooling bsw-catalog`; `--probe` also compiles and links typed
function pointers against the existing host runtime. `--write` is for explicitly
reviewed runtime changes, followed by recompiling the workbench. Product parsing
does not read fixture expectations or call the inventory materializer.

BSW declarations and definitions are mapped to their source files. Component
headers, ECU/RTE/BSW adapters and the application implementation are handled
in stories 4.11, 4.13 and 4.15 respectively. W2 plan validation checks the input
relationships; linking the W3 ECU, SC1 and ARTI behavior, C quality and
independent engineering handoff require separate checks.

`PlanDiagnostic` distinguishes input errors, unsupported target choices, missing
external dependencies and tool/source-inventory errors. It carries the logical
file, complete object path, cause and remedy when applicable. Rejection produces
no plan and writes no engineering output.

The W2 contract consumer is `plan.component_contract_files()`. It returns a
read-only deterministic file collection containing shared RTE types/statuses,
the application and service caller's headers, provenance and integrity records.
Use `contracts.preview(output)` and then
`contracts.generate_previewed(output, &preview.revision)` to install exactly
the reviewed bytes with the existing generator's output protection and backup.
The contract contains declarations only. Application compilation checks these
interfaces; ECU/RTE linking and runtime behavior are verified separately.
