# Legacy host ECU source package

Target: `{{TARGET}}`. This package uses the configured C99 host profile; the FreeRTOS ECU integration profile is documented separately. It contains BSW sources, generated configuration, a fixed build target and standalone engineering tools that use the Python standard library. Autsaro runtime, tool and template code is licensed under Apache-2.0; see LICENSE and NOTICE in this project. User-provided configurations and application code retain their own licensing. Official AUTOSAR archives and compiler binaries are excluded. AUTOSAR conformance certification is outside this package's scope.

## Independent native build

Use Python 3.11 or newer and the compiler/binutils identities recorded in `target.json`. Set `AUTOSAR_CC`, `AUTOSAR_OBJDUMP` and `AUTOSAR_GIT` to their absolute executable paths. This build does not require a checkout, uv, Rust, npm or the workbench.

From this source directory, using a supported Python interpreter:

```text
<python> tools/ecu-tool.py build --project . --output ../build --mode host
```

The build directory must be new or empty and outside the sealed source package. The tool checks the source manifest and hashes, rejects links, reparse points and extra files, and runs the pinned native compiler and binutils in a managed process scope. It installs `{{BINARY}}` only if the destination is unused. Failures return a nonzero exit code and preserve build diagnostics. Keep binaries, keys, state, logs and unlisted source files outside the source directory.

## Run the native host

Run from this source directory after the independent build:

```text
{{RUN_COMMAND}}
```

{{RUN_NOTES}}

The native protocol accepts the legacy host commands documented in the workbench runtime guide. After building, run behavior verification separately. Its results apply to the configured host behavior and the test vectors executed.

## Original input and reproduction

{{INPUT_NOTE}}

SHA-256 checks file integrity; publisher authentication requires a signature. Keep the package's tool version and target identity intact. Reimport checks the versioned input and resource contract, reconstructs the original saved inputs, and compares all regenerated source files. Compilation is a separate step, and the original package is preserved.
