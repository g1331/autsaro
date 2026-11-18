"""Links for tests: symlinks, with junctions where Windows has no symlink privilege."""

from __future__ import annotations

import os
import unittest
from pathlib import Path


def make_link(test: unittest.TestCase, link: Path, target: str | Path, *, directory: bool,
              junction: bool = False) -> None:
    """Create a link that leads to `target`, written as given; skip explicitly if impossible.

    A relative target is normalized to the host separator: Windows refuses to follow a
    symlink whose relative target is spelled with forward slashes. `junction=True` makes a
    directory junction, which Windows creates without privilege and only on Windows.
    """
    link.parent.mkdir(parents=True, exist_ok=True)
    if junction:
        if os.name != "nt" or not directory:
            test.skipTest("junctions are Windows directory links")
        import _winapi

        _winapi.CreateJunction(str(Path(link.parent, target)), str(link))
        return
    try:
        os.symlink(os.path.normpath(target), link, target_is_directory=directory)
    except OSError as error:
        if directory and os.name == "nt" and Path(target).is_absolute():
            import _winapi

            _winapi.CreateJunction(str(target), str(link))
            return
        test.skipTest(f"cannot create a symbolic link here: {error}")
