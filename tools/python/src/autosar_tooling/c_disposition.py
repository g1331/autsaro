"""Bind a currently verified source disposition to each diagnostic fingerprint."""

import json
from collections import defaultdict
from pathlib import Path

from ecu_tools.build import refuse_links

from autosar_tooling import c_fingerprint, c_retention, c_standard

DIRECTORY = "docs/standards/c-dispositions"
FORMAT = "c-diagnostic-dispositions-v1"
WINDOWS = "windows-x64-controlled-v1"
WINDOWS_DISPOSITION_BINARIES = (
    "ea592e1e4f242fb55cd5530d5496e1ddfaa6e655ebcf24bae9ee5ef6cd4ce4eb",
    "45b1a25de3df9deefa62e704f21685a6f03803a0bcee86141f3437b38d05bf0b",
)


def load(root: Path, target: str, sample: str, binary: str | None = None) -> dict[str, str]:
    if target not in c_retention.TARGETS or sample not in c_retention.PROFILES.values():
        raise ValueError("Unknown diagnostic disposition target or sample")
    directory = root / DIRECTORY / target
    if target == WINDOWS:
        if binary not in WINDOWS_DISPOSITION_BINARIES:
            # A new build has no approved fingerprints yet.
            return {}
        directory /= binary
    path = directory / (sample + ".json")
    refuse_links(path, root)
    value = json.loads(path.read_text(encoding="utf-8"),
                       object_pairs_hook=c_standard.unique_assessment_members)
    if (set(value) != {"format", "target", "sample", "proofs"}
            or (value["format"], value["target"], value["sample"]) != (FORMAT, target, sample)):
        raise ValueError(f"Diagnostic disposition matrix identity differs: {path}")
    bindings = {}
    for proof, fingerprints in value["proofs"].items():
        if not c_fingerprint.SHA256.fullmatch(proof) or not isinstance(fingerprints, list) or not fingerprints:
            raise ValueError(f"Invalid diagnostic disposition proof: {path}")
        for fingerprint in fingerprints:
            if not c_fingerprint.SHA256.fullmatch(fingerprint) or fingerprint in bindings:
                raise ValueError(f"Invalid or duplicate diagnostic disposition fingerprint: {path}")
            bindings[fingerprint] = proof
    return bindings


def proof(occurrence: dict, authorities: dict[str, dict], mandatory: set[str]) -> str | None:
    rule = occurrence["rule"]
    record = occurrence["record"]
    # The pinned checker can report the C90 implicit-declaration construct
    # under C99. The existing checked C99 applicability record is no deviation.
    c99 = (rule == "misra-c2012-17.3" and record == {
        "file": "docs/standards/os/implementation.md", "text": "## C90隐式声明规则与当前编译",
    })
    if rule in mandatory and not c99:
        return None
    ids = [occurrence.get("contextId"), occurrence.get("formalContextId")]
    contexts = [authorities.get(identifier) for identifier in ids if identifier is not None]
    if not contexts or any(row is None or row["formalStatus"] != "approved" for row in contexts):
        return None
    return c_fingerprint.sha({
        "authority": [{key: row[key] for key in ("id", "source", "evidence", "records", "formalStatus")}
                      for row in contexts],
        "disposition": {key: occurrence[key] for key in ("rule", "status", "entity", "record", "reason")},
        "category": "c99-checker-applicability" if c99 else "recorded-disposition",
    })


def apply(summary: dict, result: dict, rows: list[dict], bindings: dict[str, str],
          authorities: dict[str, dict]) -> None:
    """Verify each program independently, then summarize the position's disposition."""
    c_fingerprint.validate_occurrences(summary, rows)
    positions = defaultdict(list)
    for row in rows:
        positions[row["diagnostic"], row["locationIndex"]].append(row)
    mandatory = {"misra-c2012-" + row["id"][1:] for row in summary["coverage"]["rules"]
                 if row["category"] == "Mandatory"}
    accepted, candidates, invalid_formal_rules = set(), {}, set()
    for occurrence in result["occurrences"]:
        if occurrence["assessment"] != "accepted":
            continue
        key = occurrence["diagnostic"], occurrence["locationIndex"]
        recorded = proof(occurrence, authorities, mandatory)
        valid = []
        for row in positions[key]:
            fingerprint = row["fingerprint"]
            if fingerprint is not None and (
                    occurrence["reason"] == "grp_disapplied" and occurrence["rule"] not in mandatory
                    or recorded is not None and bindings.get(fingerprint) == recorded):
                accepted.add(fingerprint)
                valid.append(row["program"])
            elif fingerprint is not None and recorded is not None:
                candidates[fingerprint] = recorded
        if set(valid) != set(occurrence["programs"]):
            if occurrence.get("formalContextId"):
                invalid_formal_rules.add("R" + occurrence["rule"].removeprefix("misra-c2012-"))
            occurrence.update(assessment="blocked", reason="fingerprint_disposition_missing_or_stale",
                              evidenceClosure="not_established")
            result["objects"].append({
                "kind": "diagnostic", "id": occurrence["diagnostic"], "rule": occurrence["rule"],
                "reason": occurrence["reason"], "location": occurrence["location"],
                "entity": occurrence["entity"], "record": occurrence["record"],
                "programs": sorted(set(occurrence["programs"]) - set(valid)),
            })
    if invalid_formal_rules:
        obligations = result["checkerObligations"]
        counts = obligations["statusCounts"]
        for rule in sorted(invalid_formal_rules):
            del obligations["retainedRules"][rule]
            result["objects"].append({"kind": "checker-obligation", "id": rule,
                                      "reason": "fingerprint_disposition_missing_or_stale"})
        counts["not_assessed"] = counts.get("not_assessed", 0) + len(invalid_formal_rules)
        counts["retained"] -= len(invalid_formal_rules)
        if counts["retained"] == 0:
            del counts["retained"]
        obligations["conformanceAssessment"] = "not_established"
    result["fingerprintDispositions"] = {
        "format": FORMAT, "bindingsSha256": c_fingerprint.sha(bindings),
        "accepted": sorted(accepted), "stale": sorted(bindings.keys() - accepted),
        "candidates": dict(sorted(candidates.items())),
    }
    if bindings.keys() - accepted:
        result["objects"].append({"kind": "context", "id": "diagnostic-fingerprints",
                                  "reason": "registered_disposition_missing_or_stale"})
