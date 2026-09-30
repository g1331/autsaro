"""Install the pinned automotive skill bundles into this repository."""

import argparse
import ast
import hashlib
import io
import json
import shutil
import tempfile
import urllib.request
import zipfile
from pathlib import Path, PurePosixPath

from automotive_skill_adapters import patch

ROOT = Path(__file__).resolve().parents[1]
REPO = "jherrodthomas/automotive-skills-suite"
PIN = "6ad8818b6a566e0c2f5c977d8038a579f9fed9f1"
MANIFEST = ROOT / "scripts/automotive-skills-lock.json"
GROUPS = {
    "traceability-matrix": "需求、规范依据、设计、测试与结果的双向追踪",
    "test-case-catalog": "测试前置条件、输入、独立预期和通过条件",
    "verification-plan": "阶段验证方法、环境和验收条件",
    "uds-services": "UDS 服务、会话、DID/RID、NRC 和时序规格",
    "dtc-catalog": "DTC、状态及关联诊断数据目录",
    "dem-config": "Dem 事件、去抖、恢复和存储映射规格",
    "arxml-system": "Classic ECU、信号/PDU/帧及系统映射规格",
    "autosar-swc": "Classic SWC 端口、类型、runnable 和事件规格",
    "autosar-composition": "Classic 组件实例、连接和端口映射规格",
    "autosar-rte-mapping": "Classic 应用、通信与 OS 的 RTE 映射规格",
    "autosar-bsw-config": "Classic BSW 模块、参数、依赖和调度规格",
    "secure-coding-guidelines": "适用安全编码规则、工具与偏离处理规格",
}


def digest(data):
    return hashlib.sha256(data).hexdigest()


def fetch(path, ref):
    url = f"https://raw.githubusercontent.com/{REPO}/{ref}/{path}"
    with urllib.request.urlopen(url, timeout=60) as response:
        return response.read()


def extract(data, name, target):
    """Validate the complete archive before writing any member."""
    with zipfile.ZipFile(io.BytesIO(data)) as archive:
        members = []
        seen = set()
        for info in archive.infolist():
            # ZipInfo normalizes backslashes on Windows; inspect original bytes' name.
            path = PurePosixPath(info.orig_filename)
            if (
                path.is_absolute()
                or ".." in path.parts
                or "\\" in info.orig_filename
                or "\x00" in info.orig_filename
                or any(":" in part for part in path.parts)
                or not path.parts
                or path.parts[0] != name
                or (info.external_attr >> 16) & 0o170000 == 0o120000
            ):
                raise ValueError(f"Unsafe archive member: {info.filename}")
            relative = Path(*path.parts[1:])
            if info.is_dir():
                continue
            key = relative.as_posix().casefold()
            if key in seen or not relative.parts:
                raise ValueError(f"Conflicting archive member: {info.filename}")
            seen.add(key)
            members.append((relative, archive.read(info)))
        if not any(path.as_posix() == "SKILL.md" for path, _ in members):
            raise ValueError(f"No SKILL.md in {name}")
        for relative, content in members:
            destination = target / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(content)


def adapt(target, name, purpose):
    upstream = target / "references/upstream-SKILL.md"
    upstream.parent.mkdir(exist_ok=True)
    original = (target / "SKILL.md").read_bytes()
    upstream.write_bytes(original)
    reviewer = name.endswith("-checklist-reviewer")
    action = "审阅" if reviewer else "编制"
    description = f"{action}{purpose}工作簿。仅在需要这类规格工件或文档审阅时使用；普通实现任务不要求生成 Excel。"
    scripts = sorted((target / "scripts").glob("generate_*.py"))
    if len(scripts) != 1:
        raise ValueError(f"Ambiguous entry script in {name}: {scripts}")
    script = scripts[0].name
    input_name = "input.xlsx" if reviewer else "input.json"
    body = f"""---
name: {name}
description: '{description}'
---

# {purpose}：{action}

本技能输出 Excel 规格或审阅报告，不实现 C 协议栈，不证明标准符合性。
先读取原始使用接口 [upstream-SKILL.md](references/upstream-SKILL.md)，使用其中的输入字段、脚本及相关参考资料；该文件保留上游原文，其中宽泛触发、强制 Excel 和合规宣传不作为本仓库执行约定。

## 仓库使用约定

- 开发流程、审查、验收、状态及流程工件完全按仓库安装的 BMad。本技能只服务于明确委托的规格文档任务；普通开发不因此生成工作簿、报告或矩阵。
- 明确的规格文档任务按其指定输入、内容和输出路径执行。模板、脚本和工作簿是该任务的工具，不成为普通开发的附加流程；测试中间文件放临时目录。
- 本仓库目标为 R24-11；上游 AUTOSAR 模板主要基于 R22-11。参数、接口和条款必须对照适用的 R24-11 官方资料，未核实项明确标记，不替换字符串冒称完成版本迁移。
- 不猜测 ASIL/CAL、硬件、工具链和已支持范围；样例或固定模板不是项目事实。缺少必要输入时定位缺口，不用样例补齐实际结论。
- 审阅结果区分内容检查、结构检查和待确认判断；草稿评分和上游覆盖率阈值不成为项目验收门。缺证据保持未评估或待确认，生成报告成功不等于测试通过。不得从工作簿评分推导 AUTOSAR/MISRA/功能安全认证。
- 结论用简体中文，保留上游 JSON/工作表接口。报告前复核输出中的示例、固定模板内容和评分来源，不能只依据进程退出码判断规格正确。

## 执行

从仓库根目录指定专用 Python、技能脚本和明确输出路径（下列路径是占位示例，需替换为实际输入/输出）：

```powershell
& .\\.automotive-skills-venv\\Scripts\\python.exe .agents/skills/{name}/scripts/{script} <{input_name}> <output.xlsx>
```

依赖安装见 `docs/project/OWNER_GUIDE.md`。不向技能目录输出工件。普通生成/审阅无需 Office；仅确需公式重算时检查无头工具，缺少时声明未重算，不启动可见窗口。
"""
    (target / "SKILL.md").write_text(body, encoding="utf-8")


def file_digest(path, normalize=True):
    data = path.read_bytes()
    if normalize and (
        path.suffix in {".py", ".md", ".json", ".txt", ".yaml", ".yml"}
        or path.name == "LICENSE"
    ):
        data = data.replace(b"\r\n", b"\n")
    return digest(data)


def inventory(target, normalize=True):
    return {
        p.relative_to(target).as_posix(): file_digest(p, normalize)
        for p in sorted(target.rglob("*"))
        if p.is_file() and "__pycache__" not in p.parts
    }


def adapter_inventory():
    files = [
        Path(__file__),
        ROOT / "scripts/automotive_skill_adapters.py",
        ROOT / "scripts/requirements-automotive-skills.txt",
    ]
    files.extend((ROOT / "scripts/automotive_skill_patches").glob("*.py"))
    return {p.relative_to(ROOT).as_posix(): file_digest(p) for p in sorted(files)}


def validate(lock):
    for name, record in lock["skills"].items():
        target = ROOT / ".agents/skills" / name
        if (
            inventory(
                target, normalize=lock.get("file_hash_mode") == "lf-normalized-text"
            )
            != record["files"]
        ):
            raise ValueError(
                f"Local files differ from lock; refusing overwrite: {name}"
            )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--ref", default=PIN, help="Explicit upstream commit for installation/update"
    )
    parser.add_argument(
        "--update", action="store_true", help="Replace a pristine recorded install"
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Check local install without network or writes",
    )
    args = parser.parse_args()
    if len(args.ref) != 40 or any(c not in "0123456789abcdef" for c in args.ref):
        parser.error("--ref must be a full lowercase commit SHA")
    old = (
        json.loads(MANIFEST.read_text(encoding="utf-8")) if MANIFEST.exists() else None
    )
    if old:
        validate(old)
        if args.check or (old["commit"] == args.ref and not args.update):
            if old.get("adapter_files") != adapter_inventory():
                parser.error(
                    "Installation adapters changed; review and reapply with --update"
                )
            print(f"Verified {len(old['skills'])} installed skills ({old['commit']})")
            return
        if not args.update:
            parser.error(
                "A different commit requires --update and review of resulting diff"
            )
    elif args.check:
        parser.error("No recorded installation")
    names = [
        base + suffix
        for base in GROUPS
        for suffix in ("-builder", "-checklist-reviewer")
    ]
    for name in names:
        if (ROOT / ".agents/skills" / name).exists() and (
            not old or name not in old["skills"]
        ):
            raise ValueError(f"Unrecorded skill already exists: {name}")
    lock = {
        "repository": REPO,
        "file_hash_mode": "lf-normalized-text",
        "commit": args.ref,
        "adaptation": "Repository SKILL.md; original retained in references/upstream-SKILL.md; see docs/project/automotive-skills.md for patches",
        "skills": {},
        "adapter_files": adapter_inventory(),
    }
    with tempfile.TemporaryDirectory(prefix="autosar-skills-") as directory:
        stage = Path(directory)
        license_bytes = fetch("LICENSE", args.ref)
        for base, purpose in GROUPS.items():
            for suffix in ("-builder", "-checklist-reviewer"):
                name = base + suffix
                data = fetch(f"skills/{name}.skill", args.ref)
                if (
                    old
                    and old["commit"] == args.ref
                    and digest(data) != old["skills"][name]["bundle_sha256"]
                ):
                    raise ValueError(f"Pinned bundle checksum changed: {name}")
                target = stage / name
                extract(data, name, target)
                (target / "LICENSE").write_bytes(license_bytes)
                patch(target, name)
                adapt(target, name, purpose)
                for source in target.rglob("*.py"):
                    ast.parse(source.read_bytes(), filename=source.name)
                lock["skills"][name] = {
                    "bundle_sha256": digest(data),
                    "files": inventory(target),
                }
        # Recheck after network I/O, then keep a recoverable copy until complete.
        if old:
            validate(old)
        backups = stage / "backups"
        backups.mkdir()
        for name in names:
            target = ROOT / ".agents/skills" / name
            if target.exists():
                if not old or name not in old["skills"]:
                    raise ValueError(
                        f"Unrecorded skill appeared during download: {name}"
                    )
                shutil.copytree(target, backups / name)
        touched = []
        try:
            for name in names:
                target = ROOT / ".agents/skills" / name
                touched.append(name)
                target.mkdir(parents=True, exist_ok=True)
                if old:
                    for relative in old["skills"][name]["files"]:
                        (target / relative).unlink()
                shutil.copytree(stage / name, target, dirs_exist_ok=True)
            MANIFEST.write_text(
                json.dumps(lock, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
            )
        except Exception:
            # Only directories owned by this transaction are removed/restored.
            for name in touched:
                target = ROOT / ".agents/skills" / name
                if target.resolve().parent != (ROOT / ".agents/skills").resolve():
                    raise ValueError(f"Unexpected target during rollback: {target}")
                shutil.rmtree(target)
                if (backups / name).exists():
                    shutil.copytree(backups / name, target)
            if old:
                MANIFEST.write_text(
                    json.dumps(old, indent=2, ensure_ascii=False) + "\n",
                    encoding="utf-8",
                )
            raise
    print(f"Installed {len(names)} skills at {args.ref}")


if __name__ == "__main__":
    main()
