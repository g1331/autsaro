# Legacy host ECU source package

Target: `{{TARGET}}`. This is the configured C99 host profile, not the FreeRTOS ECU integration profile or an AUTOSAR conformance claim. The sealed source package contains its real BSW sources, generated configuration, fixed build target and standalone standard-library engineering tools. Product and tool bytes are supplied for owner-authorized internal use; no new public license grant is made. Official AUTOSAR archives and compiler binaries are not redistributed.

## Independent native build

Use CPython 3.12.9 and the compiler/binutils identities recorded in `target.json`. Set `AUTOSAR_CC`, `AUTOSAR_OBJDUMP` and `AUTOSAR_GIT` to their absolute executable paths. This build does not require a checkout, uv, Rust, npm or the workbench.

From this source directory, using the pinned interpreter:

```text
<CPython3.12> tools/ecu-tool.py build --project . --output ../build --mode host
```

The build directory must be new or empty and outside the sealed source package. The tool verifies the exact source closure and hashes, refuses links/reparse points and extra files, invokes the real fixed native compiler and binutils through an owned process scope, and installs `{{BINARY}}` without overwriting an existing artifact. Failures are nonzero and preserve real build diagnostics. Do not add binaries, keys, state, logs or unlisted source files to this directory.

## Run the actual native host

Run from this source directory after the independent build:

```text
{{RUN_COMMAND}}
```

{{RUN_NOTES}}

The native protocol accepts the legacy host commands documented in the workbench runtime guide. Host verification covers only the configured native behavior and exercised vectors; neither generating this package nor building it is a behavioral verification result.

## Original input and reproduction

{{INPUT_NOTE}}

SHA-256 detects accidental modification, not publisher authenticity. Keep the package's tool version and target identity intact. Reimport follows the versioned input and resource contract described above, reconstructs the original saved inputs, and compares the complete regenerated source closure. It does not silently run a compiler or rewrite an old package.
