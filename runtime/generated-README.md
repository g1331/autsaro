# Generated Windows host ECU

This directory is a self-contained C99 source project for one virtual ECU. It targets Windows with MinGW GCC and does not require the AUTOSAR workbench repository to build. It does not provide a real MCU driver, a third-party BSW interface, or AUTOSAR conformance evidence.

## Build

Install MinGW GCC and PowerShell. From any working directory, run `build.ps1` with PowerShell:

```powershell
pwsh -NoProfile -File "<generated-directory>\build.ps1"
```

On Windows PowerShell 5.1, use `powershell -NoProfile -ExecutionPolicy Bypass -File "<generated-directory>\build.ps1"` instead. Replace `<generated-directory>` with the actual path. The script uses `gcc` from `PATH`, or the compiler named by `AUTOSAR_CC`. It compiles all `src/*.c` and `Ecu_Config.c` with C99, `-Wall -Wextra -Werror -pedantic`, `include/`, and Windows `bcrypt`; the result is `ecu_host.exe` in this directory. An existing executable is never replaced. Move it yourself before rebuilding. Keep all files listed in `files.list` together; `files.sha256` records their contents for regeneration checks, not authentication.

## Run

From this directory:

```powershell
{{RUN_COMMAND}}
```

{{RUN_NOTES}}

The program reads one command per stdin line. `T <milliseconds>` advances absolute virtual time; `R <decimal CAN ID> <DLC> <uppercase hex bytes>` injects a frame; `S <signal ID> <value>` changes a Tx signal; `G <signal ID>` reads a signal. Output `X <decimal CAN ID> <DLC> <uppercase hex bytes>` is a transmitted frame, `V` is a signal value and validity, and `E` is an error. `profile.txt` maps generated frame and signal IDs to the saved configuration, and lists diagnostic IDs and options when configured. Each ECU process needs its own state files.

The ARXML sources are not included in this directory. Retain them separately if you need to edit or regenerate this project. A successful build or virtual run establishes only the behavior observed on this Windows host target.
