# Mainline integration plan

`build_plan_native` is the all-or-nothing constructor for the mainline
`singlecore-ecu-v1` profile. It receives original `InputSource` bytes and the
product's built-in `DefinitionCatalog`; official XSD/MOD archives are not
inputs. The result has private fields, no unchecked constructor, no mutable
accessor and no `Deserialize` implementation. Consumers borrow `description()`
and `sources()`.

`Workspace::integration_plan` uses the same constructor for the current
in-memory inputs. It rejects external edits to saved sources and derives
portable file identities below their common input root. The plan's raw-byte
identities include unsaved edits. Saving those edits and generating an ECU are
separate operations.

The preflight `InputInspection` runs the built-in structural and definition rules and checks typed references.
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

The producer symbol inventory (`catalog.rs`) is derived for each plan from the
selected Com and mode contracts; immutable BSW bytes and their identities come
from the trusted asset inventory (`runtime/contracts/assets-v1.json`). Product
parsing does not read fixture expectations or call the inventory materializer.

BSW declarations and definitions are mapped to their source files. Plan
validation checks the input relationships; linking the ECU, SC1 and ARTI
behavior, C quality and independent engineering handoff require separate
checks.

`PlanDiagnostic` distinguishes input errors, unsupported target choices, missing
external dependencies and tool/source-inventory errors. It carries the logical
file, complete object path, cause and remedy when applicable. Rejection produces
no plan and writes no engineering output.

The component contract consumer is `plan.component_contract_files()`. It returns a
read-only deterministic file collection containing the mainline
`Platform_Types.h`/`Std_Types.h`, shared RTE types/statuses, the component
headers, provenance and integrity records.
Use `contracts.preview(output)` and then
`contracts.generate_previewed(output, &preview.revision)` to install exactly
the reviewed bytes with the existing generator's output protection and backup.
The contract contains declarations only. Application compilation checks these
interfaces; ECU/RTE linking and runtime behavior are verified separately.
