# Rebuildable dual-host reference package

`Alpha/` and `Beta/` each contain a same-version ARXML handoff and sealed C99 source. The root `files.list`/`files.sha256` covers both members, the fixed independent `vectors.json`, target metadata and stdlib-only engineering tools. The reference covers 11-bit Classical CAN and physical DoCAN; it is not an MCU target or an AUTOSAR conformance certificate.

## Produce

From a configured developer checkout, select the target explicitly and provide the licensed R24-11 XSD archive:

```text
cargo run --manifest-path core/Cargo.toml --bin package_host_reference -- "<new-reference-directory>" --target windows-x64-controlled-v1 --xsd-archive "<absolute-licensed-XSD-archive>"
```

Use `linux-x64-controlled-v1` for Linux. Preparing the package renders sources only; it does not compile or mark native behavior as verified. Move the entire package, not individual generated files.

## Verify outside the checkout

The receiver needs the pinned CPython 3.12.9 interpreter, the target's GCC/binutils distribution and Git. Set `AUTOSAR_CC`, `AUTOSAR_OBJDUMP` and `AUTOSAR_GIT` to absolute executable paths. Compiler identities are recorded in each member's `target.json`/`toolchain.json`. Windows packages execute only on Windows x64; Linux packages execute only on Linux x64. Rust, uv, Node and the source checkout are not receiver dependencies.

```text
<CPython3.12.9> "<bundle>/tools/ecu-tool.py" verify --project "<bundle>" --build-directory "<new-empty-outside-build-directory>" --report-path "<new-outside-report.json>"
```

The verifier checks the full root and member closures, builds both actual native binaries in separate output directories, runs every fixed input, and compares all output lines in order. It records actual checks and failures in the report. A nonzero exit means verification failed; an unexecuted command is not verification. Failure logs remain in the build directory. Existing output/report files and linked or reparse-point paths are refused, not overwritten.

To rebuild only one member, run `tools/ecu-tool.py build --project <Alpha-or-Beta> --output <new-empty-outside-directory> --mode host` with that member's tool entry. To reimport, choose “导入可重建主机交付包” separately for `Alpha/` and `Beta/` in the same-version workbench and explicitly configure the licensed R24-11 XSD archive. Regenerate into new empty directories and compare the exact sealed source closure.

Only the fixed host inputs and observed vectors are covered. This reference does not establish complete ECU Extract, SWC/RTE or AUTOSAR OS support, MCU timing or interrupts, third-party interoperability, ASIL or complete AUTOSAR conformance. It does not raise the support level of other configurations.
