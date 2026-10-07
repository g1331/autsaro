"""CLI compatibility for the unified diagnostics implementation."""
from autosar_tooling.diagnostics import _archive, native, report

__all__ = ["_archive", "native", "report", "workbench"]


def workbench() -> int:
    return report("desktop")
