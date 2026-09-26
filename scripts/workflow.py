"""Local, tool-neutral status and validation for the AUTOSAR development workflow."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from datetime import date
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
STATE = ROOT / "docs" / "workflow" / "state.json"
FEEDBACK = ROOT / "docs" / "workflow" / "feedback" / "active.json"
GATES = (
    "input_roundtrip",
    "artifact_closure",
    "build_static",
    "independent_behavior",
    "user_workflow",
    "spec_obligations",
)
GATE_STATUSES = {"not_run", "failed", "pending_review", "passed", "not_applicable"}
TASK_STATUSES = {"proposed", "ready", "active", "review", "blocked", "done"}
REVIEW_STATUSES = {"not_run", "pending_review", "failed", "passed"}
FEEDBACK_STATUSES = {"observed", "trial", "needs_decision"}


def valid_choice(value: object, options: set[str]) -> bool:
    return isinstance(value, str) and value in options


def repository_file(root: Path, value: object) -> bool:
    """Only repository-relative, existing files can be used as evidence."""
    if not isinstance(value, str) or not value or "\\" in value:
        return False
    path = Path(value)
    if path.is_absolute() or path.drive or ".." in path.parts:
        return False
    return (root / path).is_file()


def evidence_file(root: Path, value: object) -> bool:
    if not repository_file(root, value):
        return False
    path = Path(value)
    return path.parts[:3] == ("docs", "workflow", "evidence") and path.name != "TEMPLATE.md"


def check_review(review: object, label: str, root: Path, errors: list[str]) -> str | None:
    if not isinstance(review, dict) or not valid_choice(review.get("status"), REVIEW_STATUSES):
        errors.append(f"{label}.review.status is invalid")
        return None
    status = review["status"]
    if status == "passed":
        if not evidence_file(root, review.get("evidence")):
            errors.append(f"{label}.review passed without a workflow evidence file")
        if not isinstance(review.get("reviewer"), str) or not review["reviewer"].strip():
            errors.append(f"{label}.review needs a reviewer")
        try:
            date.fromisoformat(review["date"])
        except (KeyError, TypeError, ValueError):
            errors.append(f"{label}.review needs an ISO date")
    return status


def check_state(state: object, root: Path) -> list[str]:
    errors: list[str] = []
    if not isinstance(state, dict):
        return ["state must be a JSON object"]
    if state.get("schema_version") != 1:
        errors.append("schema_version must be 1")
    if not isinstance(state.get("stage"), str) or not state["stage"].strip():
        errors.append("stage must describe the current bounded phase")
    if not repository_file(root, state.get("map")):
        errors.append("map must point to an existing repository file")

    capabilities = state.get("capabilities")
    tasks = state.get("tasks")
    if not isinstance(capabilities, list) or not isinstance(tasks, list):
        return errors + ["capabilities and tasks must be arrays"]

    capability_ids: set[str] = set()
    for index, capability in enumerate(capabilities):
        label = f"capabilities[{index}]"
        if not isinstance(capability, dict):
            errors.append(f"{label} must be an object")
            continue
        capability_id = capability.get("id")
        if not isinstance(capability_id, str) or not capability_id:
            errors.append(f"{label}.id is required")
        elif capability_id in capability_ids:
            errors.append(f"duplicate capability ID: {capability_id}")
        else:
            capability_ids.add(capability_id)
            label = capability_id
        for field in ("name", "release", "configuration", "target", "next_gap"):
            if not isinstance(capability.get(field), str) or not capability[field].strip():
                errors.append(f"{label}.{field} is required")
        claim = capability.get("claim_level")
        if not valid_choice(claim, {"documented_behavior", "internal_supported"}):
            errors.append(f"{label}.claim_level is invalid")
        if not repository_file(root, capability.get("documented_in")):
            errors.append(f"{label}.documented_in must point to an existing file")
        review_status = check_review(capability.get("review"), label, root, errors)
        gates = capability.get("gates")
        if not isinstance(gates, dict) or set(gates) != set(GATES):
            errors.append(f"{label}.gates must contain exactly the six defined gates")
            continue
        for gate_name in GATES:
            gate = gates[gate_name]
            gate_label = f"{label}.{gate_name}"
            if not isinstance(gate, dict) or not valid_choice(gate.get("status"), GATE_STATUSES):
                errors.append(f"{gate_label}.status is invalid")
                continue
            if gate["status"] == "passed" and not evidence_file(root, gate.get("evidence")):
                errors.append(f"{gate_label} passed without a workflow evidence file")
            if gate["status"] == "not_applicable" and (
                not isinstance(gate.get("reason"), str) or not gate["reason"].strip()
            ):
                errors.append(f"{gate_label} needs a specific not_applicable reason")
            if claim == "internal_supported" and gate["status"] not in {"passed", "not_applicable"}:
                errors.append(f"{gate_label} blocks the internal_supported claim")
        if claim == "internal_supported" and review_status != "passed":
            errors.append(f"{label} needs independent review before internal_supported")

    task_by_id: dict[str, dict] = {}
    active_count = 0
    for index, task in enumerate(tasks):
        label = f"tasks[{index}]"
        if not isinstance(task, dict):
            errors.append(f"{label} must be an object")
            continue
        task_id = task.get("id")
        if not isinstance(task_id, str) or not task_id:
            errors.append(f"{label}.id is required")
        elif task_id in task_by_id:
            errors.append(f"duplicate task ID: {task_id}")
        else:
            task_by_id[task_id] = task
            label = task_id
        if not isinstance(task.get("title"), str) or not task["title"].strip():
            errors.append(f"{label}.title is required")
        status = task.get("status")
        if not valid_choice(status, TASK_STATUSES):
            errors.append(f"{label}.status is invalid")
        if status == "active":
            active_count += 1
            if not isinstance(task.get("branch"), str) or not task["branch"].strip():
                errors.append(f"{label} is active without a branch")
            if not isinstance(task.get("base_commit"), str) or not task["base_commit"].strip():
                errors.append(f"{label} is active without a base_commit")
        if status == "blocked" and (
            not isinstance(task.get("blocked_reason"), str) or not task["blocked_reason"].strip()
        ):
            errors.append(f"{label} is blocked without a reason")
        if not repository_file(root, task.get("card")):
            errors.append(f"{label}.card must point to an existing file")
        linked = task.get("capabilities")
        if not isinstance(linked, list) or not linked:
            errors.append(f"{label}.capabilities must be a nonempty array")
        else:
            for capability_id in linked:
                if not isinstance(capability_id, str) or capability_id not in capability_ids:
                    errors.append(f"{label} references unknown capability {capability_id}")
        dependencies = task.get("depends_on")
        if not isinstance(dependencies, list):
            errors.append(f"{label}.depends_on must be an array")
        evidence = task.get("evidence")
        if not isinstance(evidence, list):
            errors.append(f"{label}.evidence must be an array")
        else:
            for item in evidence:
                if not evidence_file(root, item):
                    errors.append(f"{label} references a missing evidence file: {item}")
        if valid_choice(status, {"review", "done"}) and not evidence:
            errors.append(f"{label} cannot enter {status} without evidence")
        if status == "done":
            review_status = check_review(task.get("review"), label, root, errors)
            if review_status != "passed":
                errors.append(f"{label} cannot be done without independent review")
    if active_count > 1:
        errors.append("at most one task may be active")

    for task_id, task in task_by_id.items():
        dependencies = task.get("depends_on")
        if not isinstance(dependencies, list):
            continue
        for dependency in dependencies:
            if not isinstance(dependency, str) or dependency == task_id or dependency not in task_by_id:
                errors.append(f"{task_id} has an unknown or self dependency: {dependency}")
            elif valid_choice(task.get("status"), {"ready", "active", "review", "done"}) and task_by_id[dependency].get("status") != "done":
                errors.append(f"{task_id} cannot be {task['status']} before {dependency} is done")
    return errors


def check_feedback(feedback: object, state: dict, root: Path) -> list[str]:
    errors: list[str] = []
    if not isinstance(feedback, dict):
        return ["feedback must be a JSON object"]
    if feedback.get("schema_version") != 1:
        errors.append("feedback.schema_version must be 1")
    if not repository_file(root, "docs/workflow/feedback/HISTORY.md"):
        errors.append("feedback history file is missing")
    tasks = {task["id"]: task for task in state["tasks"] if isinstance(task, dict) and isinstance(task.get("id"), str)}
    done_ids = {task_id for task_id, task in tasks.items() if task.get("status") == "done"}
    reviewed = feedback.get("reviewed_done_task_ids")
    if not isinstance(reviewed, list) or len(reviewed) != len(set(map(str, reviewed))):
        errors.append("reviewed_done_task_ids must be a unique array")
    elif any(not isinstance(task_id, str) or task_id not in done_ids for task_id in reviewed):
        errors.append("reviewed_done_task_ids must reference completed tasks")
    items = feedback.get("items")
    if not isinstance(items, list):
        return errors + ["feedback.items must be an array"]
    seen_ids: set[str] = set()
    for index, item in enumerate(items):
        label = f"feedback.items[{index}]"
        if not isinstance(item, dict):
            errors.append(f"{label} must be an object")
            continue
        feedback_id = item.get("id")
        if not isinstance(feedback_id, str) or not feedback_id.startswith("WF-") or len(feedback_id) < 4:
            errors.append(f"{label}.id must start with WF-")
        elif feedback_id in seen_ids:
            errors.append(f"duplicate feedback ID: {feedback_id}")
        else:
            seen_ids.add(feedback_id)
            label = feedback_id
        status = item.get("status")
        if not valid_choice(status, FEEDBACK_STATUSES):
            errors.append(f"{label}.status is invalid")
        if not valid_choice(item.get("severity"), {"normal", "critical"}):
            errors.append(f"{label}.severity is invalid")
        for field in ("problem", "impact"):
            if not isinstance(item.get(field), str) or not item[field].strip():
                errors.append(f"{label}.{field} is required")
        observations = item.get("observations")
        if not isinstance(observations, list) or not observations:
            errors.append(f"{label}.observations must be a nonempty array")
        else:
            for observation_index, observation in enumerate(observations):
                observed_label = f"{label}.observations[{observation_index}]"
                if not isinstance(observation, dict):
                    errors.append(f"{observed_label} must be an object")
                    continue
                task_id = observation.get("task_id")
                if task_id is not None and (not isinstance(task_id, str) or task_id not in tasks):
                    errors.append(f"{observed_label}.task_id must reference a task or be null")
                if not isinstance(observation.get("detail"), str) or not observation["detail"].strip():
                    errors.append(f"{observed_label}.detail is required")
                if "source" in observation and not repository_file(root, observation["source"]):
                    errors.append(f"{observed_label}.source must point to an existing file")
        if status == "needs_decision":
            for field in ("decision_needed", "recommendation"):
                if not isinstance(item.get(field), str) or not item[field].strip():
                    errors.append(f"{label}.{field} is required")
        if status == "trial":
            for field in ("change", "success_condition"):
                if not isinstance(item.get(field), str) or not item[field].strip():
                    errors.append(f"{label}.{field} is required")
            changed_files = item.get("change_files")
            if not isinstance(changed_files, list) or not changed_files:
                errors.append(f"{label}.change_files must be a nonempty array")
            elif any(not repository_file(root, path) for path in changed_files):
                errors.append(f"{label}.change_files must reference existing files")
            baseline = item.get("baseline_done_task_ids")
            if not isinstance(baseline, list) or len(baseline) != len(set(map(str, baseline))):
                errors.append(f"{label}.baseline_done_task_ids must be a unique array")
            elif any(not isinstance(task_id, str) or task_id not in done_ids for task_id in baseline):
                errors.append(f"{label}.baseline_done_task_ids must reference completed tasks")
    return errors


def feedback_review_reasons(feedback: dict, state: dict) -> list[str]:
    done_ids = {task["id"] for task in state["tasks"] if task["status"] == "done"}
    reasons = []
    newly_done = done_ids - set(feedback["reviewed_done_task_ids"])
    if len(newly_done) >= 3:
        reasons.append(f"上次复盘后完成了 {len(newly_done)} 张任务卡")
    for item in feedback["items"]:
        if item["status"] == "observed" and item["severity"] == "critical":
            reasons.append(f"{item['id']} 是严重反馈")
        elif item["status"] == "observed" and len(item["observations"]) >= 2:
            reasons.append(f"{item['id']} 已出现 {len(item['observations'])} 次")
        elif item["status"] == "trial":
            trial_tasks = done_ids - set(item["baseline_done_task_ids"]) - set(feedback["reviewed_done_task_ids"])
            if len(trial_tasks) >= 2:
                reasons.append(f"{item['id']} 复盘后又经过 {len(trial_tasks)} 张试行任务")
    return reasons


def load_state() -> object:
    with STATE.open(encoding="utf-8") as stream:
        return json.load(stream)


def load_feedback() -> object:
    with FEEDBACK.open(encoding="utf-8") as stream:
        return json.load(stream)


def print_status(state: dict, feedback: dict) -> None:
    print(f"当前阶段：{state['stage']}")
    print("能力档案（未执行的证据门不否定 README 中已记录的有界行为）：")
    for capability in state["capabilities"]:
        statuses = [capability["gates"][name]["status"] for name in GATES]
        passed = sum(item in {"passed", "not_applicable"} for item in statuses)
        print(f"  {capability['id']}：{capability['claim_level']}；已审证据门 {passed}/{len(GATES)}")
        print(f"    下一缺口：{capability['next_gap']}")
    print("任务：")
    for task in state["tasks"]:
        print(f"  {task['id']} [{task['status']}]: {task['title']}")
        if task["status"] == "blocked":
            print(f"    阻断：{task['blocked_reason']}")
    reasons = feedback_review_reasons(feedback, state)
    print(f"工作流反馈：{len(feedback['items'])} 条活跃；{'复盘到期' if reasons else '当前未到复盘门槛'}")
    for item in feedback["items"]:
        print(f"  {item['id']} [{item['status']}]：{item['problem']}")
    for reason in reasons:
        print(f"  - {reason}")
    for item in feedback["items"]:
        if item["status"] == "needs_decision":
            print(f"  - {item['id']} 需要用户决定：{item['decision_needed']}")
    print("向用户报告前，仍须核对 Git、源码和实际命令输出。")


def verify(scope: str) -> int:
    npm = "npm.cmd" if sys.platform == "win32" else "npm"
    commands = {
        "core": [["cargo", "test", "--manifest-path", "core/Cargo.toml"]],
        "ui": [[npm, "ci", "--prefix", "ui"], [npm, "run", "build", "--prefix", "ui"]],
        "desktop": [["cargo", "build", "--manifest-path", "src-tauri/Cargo.toml"]],
        "all": [
            [sys.executable, "-B", "-m", "unittest", "discover", "-s", "scripts", "-p", "test_workflow.py"],
            [npm, "ci", "--prefix", "ui"],
            [npm, "run", "build", "--prefix", "ui"],
            ["cargo", "test", "--manifest-path", "core/Cargo.toml"],
            ["cargo", "build", "--manifest-path", "src-tauri/Cargo.toml"],
        ],
    }
    for command in [["git", "diff", "--check"], ["git", "diff", "--cached", "--check"], *commands[scope]]:
        print(f"> {' '.join(command)}", flush=True)
        try:
            result = subprocess.run(command, cwd=ROOT, check=False)
        except OSError as error:
            print(f"Cannot run {command[0]}: {error}", file=sys.stderr)
            return 1
        if result.returncode:
            print(f"Verification stopped: exit code {result.returncode}", file=sys.stderr)
            return result.returncode
    return 0


def main() -> int:
    if sys.platform == "win32":
        sys.stdout.reconfigure(encoding="utf-8")
        sys.stderr.reconfigure(encoding="utf-8")
    parser = argparse.ArgumentParser(description=__doc__)
    subcommands = parser.add_subparsers(dest="command", required=True)
    subcommands.add_parser("status", help="Read-only status index")
    subcommands.add_parser("check", help="Validate cross-agent state and evidence references")
    verify_parser = subcommands.add_parser("verify", help="Run local verification gates")
    verify_parser.add_argument("--scope", choices=("core", "ui", "desktop", "all"), required=True)
    arguments = parser.parse_args()
    try:
        state = load_state()
        feedback = load_feedback()
    except (OSError, json.JSONDecodeError) as error:
        print(f"Cannot read workflow state or feedback: {error}", file=sys.stderr)
        return 1
    errors = check_state(state, ROOT)
    if not errors:
        errors.extend(check_feedback(feedback, state, ROOT))
    if errors:
        for error in errors:
            print(f"State error: {error}", file=sys.stderr)
        return 1
    if arguments.command == "status":
        print_status(state, feedback)
    elif arguments.command == "check":
        print("Workflow state is valid.")
    else:
        return verify(arguments.scope)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
