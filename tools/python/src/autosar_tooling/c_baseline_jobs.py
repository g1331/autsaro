"""Match protected master jobs that publish raw C analysis artifacts."""


def producer_matches(name: str, target: str, sample: str) -> bool:
    windows = target.startswith("windows")
    job = "c-analysis-samples" + ("-windows" if windows else "")
    system = "windows-2025-vs2026" if windows else "ubuntu-24.04"
    legacy = f"{job} ({system}, {target}, {sample}, {sample})"
    paired = {f"{job} ({target}, {first} + {second})" for first, second in
              (("standard-ecu", "user-application"), ("component-pipeline", "no-diagnostic"))
              if sample in (first, second)}
    return (name in paired or name == legacy
            or name.endswith("...") and legacy.startswith(name[:-3]))

# The job selector moved out of the raw input set without changing the scan.
# Recognize only this exact wrapper transition, in each native checkout format.
# Remove the bridge once pre-split raw artifacts have expired.
_PREVIOUS_WRAPPERS = {
    "e0453a942d60ecf2dd37473afe43e89a9af11e21cb74a80faa65e4c426d29cf2":
        "86d510532958d716a7e9d821e726c535793b93c14ee9434f5166fcaa7916c969",
    "cde515dfae48a0936149f30bad299cea6825ca8e2a9be558529ce2f195ab5da5":
        "6a0d29b1154f687be2cef02355fbf8a2a94e737ee652ddf1ce6fa268a180205a",
}


def previous_wrapper(current_sha256: str) -> str | None:
    return _PREVIOUS_WRAPPERS.get(current_sha256)
