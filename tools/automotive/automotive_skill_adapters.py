"""Small, reproducible patches for the pinned document skill scripts."""

import re
from pathlib import Path

PATCHES = Path(__file__).with_name("automotive_skill_patches")
NOTICE = """

def _repository_notice(wb):
    import json
    from pathlib import Path
    import sys
    for sheet in wb:
        for row in sheet:
            for cell in row:
                if cell.value == "APPROVED":
                    cell.value = "DOCUMENT CHECKS COMPLETE - review required"
                elif cell.value == "CONDITIONAL APPROVAL":
                    cell.value = "DOCUMENT ISSUES - review required"
                elif cell.value in ("Internal review comments incorporated", "Released for architecture review", "Initial release"):
                    cell.value = "TEMPLATE PLACEHOLDER - not a recorded project event"
    if "03_Coding_Rules_Catalog" in wb.sheetnames:
        rules = wb["03_Coding_Rules_Catalog"]
        col = rules.max_column + 1
        rules.cell(1, col, "Repository validation status")
        for row in range(2, rules.max_row + 1):
            rules.cell(row, col, "UNVERIFIED UPSTREAM TEMPLATE - rule ID/text/severity/CWE require authoritative review")
    ws = wb.create_sheet("Repository Scope")
    ws.append(["Field", "Value"])
    ws.append(["Purpose", "Document generation/review only; NOT a runtime test or certification"])
    ws.append(["Evidence", "Separate CONTENT / STRUCTURE / DRAFT; missing evidence remains unassessed"])
    ws.append(["Version", "Upstream AUTOSAR R22-11 templates; repository R24-11 obligations need verification"])
    ws.append(["Thresholds", "Upstream thresholds are advisory, not repository acceptance gates"])
    ws.append(["Templates", "Fixed example/default content requires review; not project facts"])
    if len(sys.argv) > 1 and Path(sys.argv[1]).suffix.lower() == ".json":
        with open(sys.argv[1], encoding="utf-8") as stream:
            provenance = json.load(stream).get("provenance", {})
        for key, value in provenance.items():
            ws.append([key, json.dumps(value, ensure_ascii=False) if isinstance(value, (dict, list)) else value])
    ws.column_dimensions["A"].width = 20
    ws.column_dimensions["B"].width = 100
"""


def replace_required(path, old, new):
    text = path.read_text(encoding="utf-8")
    if old not in text:
        raise ValueError(f"Upstream patch no longer applies: {path.name}")
    path.write_text(text.replace(old, new), encoding="utf-8")


def patch(target, name):
    if name == "traceability-matrix-builder":
        path = target / "scripts/generate_trace.py"
        replace_required(
            path,
            '["Category", "Element ID", "Title", "Type", "Links To"]',
            '["Category", "Element ID", "Title", "Type", "Links To", "Reported Result"]',
        )
        replace_required(
            path,
            '", ".join(element["links"]),',
            '", ".join(element["links"]),\n                element["result"],',
        )
    if name == "traceability-matrix-checklist-reviewer":
        path = target / "scripts/trace_probe.py"
        replace_required(path, "min_col=1, max_col=5", "min_col=1, max_col=6")
        replace_required(
            path,
            '"links": row[4] if len(row) > 4 else "",',
            '"links": row[4] if len(row) > 4 else "",\n                        "result": row[5] if len(row) > 5 else "",',
        )
        path = target / "scripts/generate_checklist.py"
        replace_required(
            path,
            '"No non-conformities recorded."',
            '"No automatic document failures recorded; draft checks and execution evidence still require review."',
        )
        replace_required(
            path,
            "        wb.save(output_xlsx)",
            """        execution = wb.create_sheet("Execution Evidence")
        execution.append(["Test ID", "Reported result (not independently verified)"])
        for element in probed.elements:
            if element.get("type") == "Test":
                execution.append([element["id"], element.get("result") or "NOT REPORTED"])
        wb.save(output_xlsx)""",
        )
        replace_required(
            target / "scripts/check_definitions.py",
            'rating, finding, evidence = "LC", DRAFT_NOTE, "DRAFT"',
            'rating, finding, evidence = "NA", DRAFT_NOTE, "DRAFT"',
        )
    if name == "test-case-catalog-builder":
        (target / "scripts/generate_tc_catalog.py").write_bytes(
            (PATCHES / "generate_tc_catalog.py").read_bytes()
        )
    if name == "test-case-catalog-checklist-reviewer":
        for source, destination in (
            ("tc_probe.py", "tc_probe.py"),
            ("tc_check_definitions.py", "check_definitions.py"),
        ):
            (target / "scripts" / destination).write_bytes(
                (PATCHES / source).read_bytes()
            )
    if name == "verification-plan-checklist-reviewer":
        path = target / "scripts/check_definitions.py"
        replace_required(
            path,
            'rating = "LC"  # Default: largely compliant',
            'rating = "NA"  # Unassessed default',
        )
        replace_required(path, 'rating = "LC"\n', 'rating = "NA"\n')
        replace_required(
            path,
            '"finding": finding,',
            '"finding": "DRAFT - not machine-assessed" if rating == "NA" else "STRUCTURE - field presence only",',
        )
        replace_required(path, '"FC"', '"LC"')
    if name == "autosar-rte-mapping-checklist-reviewer":
        replace_required(
            target / "scripts/check_definitions.py",
            'return CheckResult("FC", "Task priorities analyzed for inversion")',
            'return CheckResult("NA", "DRAFT - priority values alone do not prove inversion analysis")',
        )
        replace_required(
            target / "scripts/check_definitions.py",
            'return CheckResult("LC", "Trigger latencies should be documented")',
            'return CheckResult("NA", "DRAFT - trigger latency not assessed")',
        )
        replace_required(
            target / "scripts/check_definitions.py",
            'return CheckResult("LC", "Stack sizing allocation for all tasks required")',
            'return CheckResult("NA", "DRAFT - stack allocation not assessed")',
        )
    if name == "dtc-catalog-checklist-reviewer":
        replace_required(
            target / "scripts/generate_checklist.py", "r[0].rating", "r[2].rating"
        )
    if name == "secure-coding-guidelines-checklist-reviewer":
        path = target / "scripts/generate_checklist.py"
        replace_required(
            path,
            "from check_definitions import CHECKS",
            "from check_definitions import CHECKS\nfrom dataclasses import asdict",
        )
        replace_required(
            path, "results[check.id] = result", "results[check.id] = asdict(result)"
        )
        replace_required(path, "check.category", "check.section")
        replace_required(
            path, '"Secure Coding Guidelines Assessment"', '"SCG Assessment"'
        )
        replace_required(
            path,
            "build_dashboard(summary_ws, results, project_label=",
            "dashboard_results = {}\n    for check in CHECKS:\n        dashboard_results.setdefault(check.tab, []).append((check, check.verify(probed)))\n    build_dashboard(summary_ws, dashboard_results, project_label=",
        )
        replace_required(
            target / "scripts/check_definitions.py",
            'def va_12_roles(p): return CheckResult(rating="FC" if p.approver else "PC", finding="Qualified reviewers assigned" if p.approver else "Reviewers not assigned.", confidence="High")',
            'def va_12_roles(p): return draft("Reviewer qualification cannot be inferred from an approver name.")',
        )
    if name == "secure-coding-guidelines-builder":
        replace_required(
            target / "scripts/generate_secure_coding_guidelines.py",
            "project = json.load(f)",
            'project = json.load(f)\n        if isinstance(project.get("project"), dict):\n            project = {**project.pop("project"), **project}',
        )
    if name.startswith("secure-coding-guidelines-"):
        for path in (target / "scripts").glob("*.py"):
            text = path.read_text(encoding="utf-8")
            path.write_text(
                text.replace(
                    "08_Authentication_Authorization_Rules",
                    "08_Auth_Authorization_Rules",
                ),
                encoding="utf-8",
            )
    if name == "verification-plan-checklist-reviewer":
        replace_required(
            target / "scripts/vplan_probe.py",
            'probed.scope = str(row[0]) if row[0] else ""',
            'probed.scope = str(ws["A7"].value or "")',
        )
    for path in (target / "scripts").glob("*.py"):
        text = path.read_text(encoding="utf-8")
        # JSON reads must work with Chinese input on Windows as well.
        text = re.sub(
            r'with open\((input_json|input_path|input_file|sys\.argv\[1\])(?:, "r")?\) as f:',
            r'with open(\1, encoding="utf-8") as f:',
            text,
        )
        if path.name.startswith("generate_"):
            text, count = re.subn(
                r"^( +)wb\.save\(",
                r"\1_repository_notice(wb)\n\1wb.save(",
                text,
                flags=re.MULTILINE,
            )
            if count:
                text = text.replace(
                    'if __name__ == "__main__":',
                    NOTICE + '\n\nif __name__ == "__main__":',
                )
        path.write_text(text, encoding="utf-8")
