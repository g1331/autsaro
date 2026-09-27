# Fixed Windows host reference bundle

This is a bounded, reproducible reference for two virtual ECUs. `Alpha/` and `Beta/` each contain a rebuildable ARXML handoff and generated C99 source. `vectors.json` gives independent expected CAN and physical diagnostic lines. `verify.ps1` checks the package, builds both ECUs in a temporary directory, runs the vectors with timeouts, and writes a JSON report outside the package.

## Produce

From the same-version workbench checkout, with a legally obtained R24-11 XSD archive in the location documented by the workbench, run:

```powershell
cargo run --manifest-path core/Cargo.toml --bin package_host_reference -- "<new-output-directory>"
```

The bundle contains the source ARXML; it does not contain the workbench, XSD, GCC, or PowerShell. Review the ARXML before sharing. The listed SHA-256 digests detect accidental modification but do not authenticate the publisher.

## Verify after moving

On Windows with PowerShell and MinGW GCC on `PATH`, move the whole bundle to a new location and run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File "<bundle-directory>\verify.ps1" -ReportPath "<outside-bundle>\reference-result.json"
```

The report records the actual checks and tool versions. A nonzero exit means failed verification; if the script was not run, the behavior remains unverified. To reimport, choose “导入可重建主机交付包” separately for `Alpha/` and `Beta/` in the same-version workbench, after preparing the R24-11 XSD archive. Regenerate to new empty directories, compare the C99 source and configuration files, and use `build.ps1` to build a single ECU. The verifier builds the delivered source without the workbench.

This confirms only the fixed Windows host inputs and observed vectors. It does not establish complete ECU Extract, SWC/RTE/AUTOSAR OS support, MCU timing or interrupts, persistent storage, third-party interoperability, or complete AUTOSAR conformance. It does not raise the support level of other configurations.
