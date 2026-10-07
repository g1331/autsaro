# Rebuildable dual-host reference package

`Alpha/` and `Beta/` each contain an ARXML handoff and sealed C99 source for the same tool version. The root `files.list`/`files.sha256` covers both members, the fixed independent `vectors.json`, target metadata and engineering tools that use only the Python standard library. The reference supports 11-bit Classical CAN and physical DoCAN on the declared host target. MCU support and AUTOSAR conformance certification are outside its scope.

## Produce

From a configured developer checkout, select the target explicitly and provide the licensed R24-11 XSD archive:

```text
cargo run --manifest-path core/Cargo.toml --bin package_host_reference -- "<new-reference-directory>" --target windows-x64-controlled-v1 --xsd-archive "<absolute-licensed-XSD-archive>"
```

Use `linux-x64-controlled-v1` for Linux. Package preparation renders source files; compilation and behavior verification are separate steps. Move the package as a whole to retain its inputs, metadata and tools.

## Verify outside the checkout

The receiver needs the pinned CPython 3.12.9 interpreter, the target's GCC/binutils distribution and Git. Set `AUTOSAR_CC`, `AUTOSAR_OBJDUMP` and `AUTOSAR_GIT` to absolute executable paths. Compiler identities are recorded in each member's `target.json`/`toolchain.json`. Windows packages execute only on Windows x64; Linux packages execute only on Linux x64. Rust, uv, Node and the source checkout are not receiver dependencies.

```text
<CPython3.12.9> "<bundle>/tools/ecu-tool.py" verify --project "<bundle>" --build-directory "<new-empty-outside-build-directory>" --report-path "<new-outside-report.json>"
```

The verifier checks all listed files at the root and in both members, builds both native binaries in separate output directories, runs every fixed input, and compares the output lines in order. The report records the checks and any failures. Verification failures return a nonzero exit code and leave logs in the build directory. Existing output or report files, symbolic links and reparse points are rejected.

To rebuild only one member, run `tools/ecu-tool.py build --project <Alpha-or-Beta> --output <new-empty-outside-directory> --mode host` with that member's tool entry. To reimport, choose “导入可重建主机交付包” separately for `Alpha/` and `Beta/` in the same-version workbench and explicitly configure the licensed R24-11 XSD archive. Regenerate into new empty directories and compare the exact sealed source closure.

Verification covers the fixed host inputs and test vectors in this package. Other configurations need their own verification. Complete ECU Extract, SWC/RTE and AUTOSAR OS support, MCU timing and interrupts, third-party interoperability, ASIL and complete AUTOSAR conformance are outside this reference's scope.

## Licensing

Autsaro runtime, tools and template code is licensed under Apache-2.0; see LICENSE and NOTICE. Each member also includes these files. User-provided configurations and application code retain their own licensing.
