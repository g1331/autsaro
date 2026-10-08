# Product

[简体中文](PRODUCT.zh-CN.md)

<!-- impeccable:product-schema 1 -->

This document records stable product facts for design work. Product direction and requirements are governed by the [BMad product brief](_bmad-output/planning-artifacts/product-brief.md) and [PRD](_bmad-output/planning-artifacts/prd.md); architecture, implementation scope, and task status remain managed in their respective BMad artifacts. This document does not establish a separate roadmap, implementation specification, or completion ledger. The maintainer confirmed the product direction and platform classification during this initialization.

## Platform

web

The actual delivery format is a local Tauri desktop workbench, with a React/TypeScript interface built using Web technologies. Here, `web` refers to the interface technology and design language; it does not imply an online service, a mobile product, or full desktop capabilities in a browser alone.

## Users

The primary users are ECU integration engineers and BSW developers. They need to combine system/ECU configuration, application interfaces, basic software, and target environments into ECU projects that can be inspected, generated, and handed over.

A receiving engineer also participates in the project handover workflow: they must be able to independently rebuild and reverify projects within the declared support scope using the delivered content and explicit dependencies, without relying on the original author's checkout, personal paths, or undocumented actions.

Learners are not the primary users in the near term. The learning layer is a product direction for after multiple categories of configuration, generation, and virtual execution capabilities have stabilized; it is not currently in the implementation queue.

## Product Purpose

Build an in-house AUTOSAR Classic basic software stack and a configuration, validation, and code-generation workbench that turns the relationships among system requirements, software components, BSW, and targets into ECU projects that are understandable, configurable, verifiable, generatable, and deliverable.

The main configuration workflow must be independently usable: after installing the software, users can use built-in validation rules and in-house module definitions to create or import real ARXML, then inspect, edit, save, validate, and generate source code. Users do not need to supply official XSD/MOD files, and installing a compiler or running a virtual ECU is not a prerequisite for using the Configurator. The current built-in configuration chain already provides this capability; compatibility paths and comparison tests against official resources still use external archives. See the README and corresponding implementations for the exact scope.

Success is not displaying source code or completing a single example build. For configurations within the declared support scope, it means delivering complete inputs, generic stack source code, generated configuration and interfaces, target dependencies, and a rebuild method, then verifying actual behavior and failure recovery against independent expectations. Users should not have to manually fill in the protocol stack or missing symbols after every generation.

## Positioning

The product provides both an in-house Classic BSW implementation and a project configuration/generation chain. It is neither an editor that merely wraps an existing commercial protocol stack nor an example tool that generates only a few files.

The product maintains generic BSW; it generates configuration and the necessary RTE/application interfaces and integration files for each ECU. Users provide the actual vehicle-function algorithms. The proportion of generated source code is not a quality metric; clear project responsibilities and independent deliverability are central.

Vector MICROSAR Classic + DaVinci Configurator and EB tresos AutoCore + Studio are references for the product format, not statements of current support scope, interoperability, or conformance.

## Operating Context

- Local project workflow: create or import multi-file ARXML for the same ECU → inspect and edit supported objects → preview differences and save → validate → preview and generate a standalone project → optionally build and verify host behavior → hand over and reimport.
- The README and actual source code define the current interface and capabilities; candidate directions are not implemented capabilities.
- ARXML, generated source code, build artifacts, and runtime state have different ownership and lifecycles. Previews do not write to disk; saving or generation happens only after confirmation. User inputs, application source code, and existing artifacts must not be silently overwritten. Unapplied drafts must be clearly distinguished from saved configuration.
- R5's normal configuration chain uses independently implemented, versioned rules bundled with the software and in-house module definitions. Official XSD/MOD files serve development cross-checks and independent comparisons, not ordinary user settings; third-party module definitions retain explicit optional import. The built-in edition/coverage and actual validation results are displayed separately. Unknown rules must not be falsely reported as passed, and corrupted bundled rules are installation/tool errors, not a reason to require users to download official archives.
- Windows and Linux have controlled native host targets and project handover records; native macOS builds and IPC have not been verified. Preparing source code for another target and executing it on the current host are distinct outcomes.
- New interfaces and verification must not take over the desktop the user is currently using. Acceptance checks involving real desktop windows and IPC use isolated sessions; when isolation is unavailable, the unverified scope is stated explicitly.

## Capabilities and Constraints

### Current Baseline

- The current baseline uses CP/FO R24-11 and supports bounded multi-file ARXML import, safe preservation of unsupported content, difference previews, saving and reopening, validation, and standalone C99 host-project generation. Accepting arbitrary ARXML input does not mean complete ECU Extract/BSW/SWC input coverage.
- Existing capabilities include an 11-bit Classical CAN signal chain, bounded physical DoCAN, Dcm, and optional Dem/host-file NvM behavior. Capacity, SID/subfunction coverage, and target differences are defined by the current implementation, [README](README.md), and corresponding story/spec. These are not complete implementations of the CAN, UDS, or storage standards.
- `Os_Advance` in the legacy host target is a virtual-time scheduler. The new Epic 4 target uses a fixed FreeRTOS kernel + in-house AUTOSAR OS semantics + limited single-core policy extensions. This direction is settled and is not to be reselected as part of interface design.
- Epic 4 has completion records for fixed Win64 host SC1 behavior and independent handover. Conclusions are tied to the actual edition, configuration, target, toolchain, and evidence. Arbitrary user projects still require reverification and do not inherit the reference project's passing results.
- Generation, preflight checks, builds, and behavioral verification are distinct states. Old binaries, partial success, or results that have not been run must not be displayed as completed verification of the current configuration.

### Confirmed Directions and Unresolved Boundaries

- R5 for the unified Configurator has been formalized as Epic 7: the [selected requirements](_bmad-output/planning-artifacts/prd.md#r5-选定范围与完成契约), [incremental architecture](_bmad-output/planning-artifacts/architecture/epic-7/ARCHITECTURE-SPINE.md), 11 Stories, and [interaction specification](_bmad-output/planning-artifacts/ux-designs/ux-Autosar-2026-10-03/EXPERIENCE.md) have been revised together to reflect the user-confirmed built-in-rules direction. See [implementation readiness](_bmad-output/planning-artifacts/implementation-readiness.md#2026-10-03-r5--epic-7-实施就绪) for the latest readiness conclusion. Implementation results are recorded in the current central implementation specification and Git; this document does not duplicate dynamic story status. R6 fixes only the shared object/reference and application-ownership seams; other candidates remain to be planned.
- The first complete reference product chain is a generic CAN + diagnostic ECU. Complete delivery must include the application interfaces, OS, lifecycle/network management, and failure recovery required by the declared configuration. An intermediate increment with always-on CAN cannot substitute for the complete reference ECU.
- Over the long term, expand effective module and target coverage for the selected Classic edition in stages. This does not require every ECU to enable every module, and historical story counts must not be used to calculate an undefined completion percentage.
- The first MCU board, drivers and licensing, toolchain, specific diagnostic/network-management profiles, input coverage, and migration rules remain subject to their respective product plans. There is currently no real MCU verification, and hardware support must not be announced in advance.
- Host execution, real-hardware execution, third-party interoperability, complete specification obligations, MISRA, hard real-time behavior, functional safety, and official conformance certification each require separate evidence; none can be inferred from another.
- Adaptive Platform is a separate future project direction; do not prebuild a shared CP/AP implementation. The learning layer will be planned later. Exercises must use the real capabilities available at that time, keep teaching projects separate from user projects, and determine passing results from actual configuration, build, or runtime checkpoints—not from completing clicks.
- Original project code and documentation use [Apache-2.0](LICENSE). Third-party redistribution rights, support commitments, and release processes still require assessment against the actual artifacts. This document makes no public-release or certification commitment.

## Brand Commitments

- The current application name is “Classic CAN 配置工作台”; its English distribution name is `Classic CAN Workbench`. The long-term product direction is a more complete Autosar Classic engineering platform; initialization must not unilaterally rename the existing application.
- Current official icons are in ui/public and src-tauri/icons, with design sources retained in the BMad UX baseline. Design snapshots are not records of current implementation or release status. Do not use official AUTOSAR trademarks.
- The icon must be recognizable as automotive electronics while remaining lively and graphically suited to a contemporary desktop brand. The user prefers flat design A (1 > 3 > 2), requires it to fit the engineering interface more closely, explicitly rejected a green adaptation, and requested gradients with a sense of material quality. JetBrains/VS Code serve as references for layering, without copying their orange-pink-purple palette. The current design retains A's shape, uses coral hues to establish lighting and tonal depth, and uses restrained solid coral for the interface's primary actions. On 2026-10-03, the user confirmed the current transparent coral-material icon and interface baseline as “provisionally finalized”; design snapshots are not records of current software implementation or release status.
- Identity graphics use a genuinely transparent background, without a white backing. The vehicle silhouette and coral layers remain consistent across light and dark themes, while the lights switch between graphite and warm white according to the effective theme. When following the system theme, synchronize the graphic without discarding unapplied settings. Generic desktop icons must work on both light and dark backgrounds, but must not claim that native ICO/ICNS icons automatically follow the theme.
- The current interface presents tasks in Simplified Chinese while preserving the accurate meaning of standard terms, module names, identifiers, commands, and original errors. Capability descriptions must be specific and verifiable, without presenting development goals as available features.
- The interface baseline retains the project tree + object table + property inspector, JetBrains-style unified menu/sidebar/bottom tool-window logic, and the ChatGPT desktop application's neutral palette and role-based colors. References should form a coherent system, not a collage of controls; normal operational interfaces must not contain design explanations. Light and dark themes share the same information hierarchy; the design specifications carry the detailed visual and interaction rules.

## Evidence on Hand

- [Product brief](_bmad-output/planning-artifacts/product-brief.md): primary users, product mechanisms, long-term candidate directions, and boundaries for the later learning layer.
- [PRD](_bmad-output/planning-artifacts/prd.md) and [architecture](_bmad-output/planning-artifacts/architecture.md): confirmed outcomes, input/generation responsibilities, and future contracts still to be decided.
- [README](README.md), [runtime documentation](runtime/README.md), and BMad story/spec: existing workflows, bounded capabilities, target dependencies, and recorded verification scope. Where older passages describe historical status, cross-check against applicable newer story/spec and current source code; do not infer all capabilities from a single overview statement.
- `ui/src/`, `src-tauri/`, `core/src/`, and `runtime/`: the existing React interface, desktop backend, configuration/generation core, and C99 runtime. `core/tests/end_to_end.rs` and fixtures hold integration-verification inputs.
- `ui/public/logo-app.png` and `src-tauri/icons/`: existing product icon assets.
- This initialization has no evidence supporting claims of real MCU support, official certification, customer endorsement, or performance benchmarks; do not fabricate such content. Historical host acceptance checks were not rerun during this initialization.

## Product Principles

1. **Real projects first.** Connect configuration through to complete delivery. Examples demonstrate interfaces; they do not replace real-project capabilities or users' vehicle algorithms.
2. **Independent configuration, explicit verification.** Decouple configuration/generation from optional compilation/execution, and apply resource limits precisely to the affected operations.
3. **Respect ownership.** Safe round trips, explicit differences, and confirmation steps protect inputs, user code, and old artifacts. Do not silently lose content, migrate, or overwrite.
4. **Evidence determines claims.** Status must correspond to the current configuration, artifacts, and actual target. Clearly present unrun, failed, and unsupported states; do not display false success.
5. **Expand coverage without weakening contracts.** Reuse existing models and the settled OS direction, expanding combination by combination. BMad remains the sole source of requirements and implementation status; later directions must not displace near-term delivery.
