"""Linux-only private network namespace with a namespace-local command supervisor."""

from __future__ import annotations

import argparse
import json
import os
import shutil
import sys
import tempfile
from collections.abc import Iterator
from contextlib import contextmanager
from pathlib import Path

from ecu_tools.owner import Owner
from ecu_tools.process import ProcessSpec, run_bounded


def executable(name: str) -> str:
    selected = shutil.which(name)
    if selected is None:
        raise RuntimeError(f"Private network isolation requires installed tool: {name}")
    return str(Path(selected).resolve(strict=True))


@contextmanager
def namespace_owner() -> Iterator[None]:
    """Replace the outer broker only while executing inside the private namespace."""
    previous = {
        name: os.environ.get(name)
        for name in ("ECU_OWNER_SOCKET", "ECU_OWNER_TOKEN", "ECU_OWNER_SCOPE")
    }
    owner = None
    try:
        # The local supervisor is a separate service, not an outer registered scope.
        os.environ.pop("ECU_OWNER_SCOPE", None)
        owner = Owner.start()
        os.environ["ECU_OWNER_SOCKET"] = str(owner.socket_path)
        os.environ["ECU_OWNER_TOKEN"] = owner.token
        yield
    finally:
        for name, value in previous.items():
            if value is None:
                os.environ.pop(name, None)
            else:
                os.environ[name] = value
        if owner is not None:
            owner.close()


def entered(args) -> int:
    current = os.readlink("/proc/self/ns/net")
    if current == args.outer_netns:
        raise RuntimeError("Offline verification did not enter a distinct network namespace")
    uid_map = Path("/proc/self/uid_map").read_text(encoding="ascii")
    mappings = [tuple(map(int, line.split())) for line in uid_map.splitlines()]
    if args.outer_uid is None or args.outer_uid <= 0 or (0, args.outer_uid, 1) not in mappings:
        raise RuntimeError("Offline app namespace must map only the ordinary outer user's identity")
    def execute(argv: list[str], scope: str):
        result = run_bounded(ProcessSpec.seconds(argv, args.evidence, 10, args.evidence, scope))
        if not result.success:
            raise RuntimeError(f"Private network observation failed: {result}")
        return result
    with namespace_owner():
        ip = executable("ip")
        execute([ip, "link", "set", "lo", "up"], "private-loopback-up")
        address = execute([ip, "-json", "address", "show"], "private-addresses")
        interfaces = json.loads(address.stdout.read_text())
        if not interfaces or any(item["ifname"] != "lo" for item in interfaces):
            raise RuntimeError("Offline namespace unexpectedly exposes a non-loopback interface")
        routes = {}
        for family in ("-4", "-6"):
            result = execute([ip, family, "-json", "route", "show"], f"private-routes{family}")
            routes[family] = json.loads(result.stdout.read_text())
            if any(route.get("dev") != "lo" for route in routes[family]):
                raise RuntimeError("Offline namespace unexpectedly exposes a non-loopback route")
        receipt = args.evidence / "network-isolation.json"
        receipt.write_text(json.dumps({
            "platform": "linux", "method": "private-user-and-network-namespace",
            "status": "enforced", "outerNamespace": args.outer_netns,
            "namespace": current, "interfaces": interfaces, "routes": routes,
            "binary": str(args.binary), "userNetworkChanged": False,
            "outerUid": args.outer_uid, "namespaceUid": os.geteuid(), "uidMap": uid_map,
        }, indent=2), encoding="utf-8")
        os.environ["AUTOSAR_NATIVE_OFFLINE_RECEIPT"] = str(receipt)
        os.environ.pop("DBUS_SESSION_BUS_ADDRESS", None)
        from autosar_tooling.desktop import run

        return run(
            "linux", args.binary, True, args.source_checkout,
            args.profile == "builtin-only",
        )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--source-checkout", type=Path, required=True)
    parser.add_argument("--evidence", type=Path)
    parser.add_argument("--profile", choices=("builtin-only", "licensed-oracle"), default="builtin-only")
    parser.add_argument("--entered-netns", action="store_true")
    parser.add_argument("--outer-netns")
    parser.add_argument("--outer-uid", type=int)
    args = parser.parse_args()
    if sys.platform != "linux":
        parser.error("Private user/network namespaces require a native Linux host")
    previous_mask = os.umask(0o077)
    try:
        args.binary = args.binary.resolve(strict=True)
        args.source_checkout = args.source_checkout.resolve()
        if args.entered_netns:
            if not args.outer_netns or args.evidence is None:
                parser.error("Entered namespace requires its original identity and owned evidence directory")
        else:
            if os.geteuid() == 0:
                parser.error("Installed ordinary-user acceptance cannot start from a host-root controller")
            args.evidence = args.evidence or Path(tempfile.mkdtemp(prefix="autosar-native-offline-"))
            args.evidence.mkdir(mode=0o700, parents=True, exist_ok=True)
        actual_mask = next(
            line.split()[1]
            for line in Path("/proc/self/status").read_text(encoding="ascii").splitlines()
            if line.startswith("Umask:")
        )
        if actual_mask != "0077":
            raise RuntimeError("Private native launcher did not establish creation mask 0077")
        permission_phase = "namespace" if args.entered_netns else "outer"
        (args.evidence / f"{permission_phase}-creation-context.json").write_text(
            json.dumps(
                {
                    "phase": permission_phase,
                    "pid": os.getpid(),
                    "uid": os.getuid(),
                    "gid": os.getgid(),
                    "namespace": os.readlink("/proc/self/ns/net"),
                    "previousMask": oct(previous_mask),
                    "actualMask": actual_mask,
                    "scope": "private-native-launcher-and-owned-children",
                },
                indent=2,
            ),
            encoding="utf-8",
        )
        if args.entered_netns:
            return entered(args)
        command = [executable("unshare"), "--user", "--map-root-user", "--net", "--fork", "--kill-child=TERM", str(Path(sys.executable).resolve()), "-m", "autosar_tooling.native_offline", "--entered-netns", "--outer-netns", os.readlink("/proc/self/ns/net"), "--outer-uid", str(os.geteuid()), "--binary", str(args.binary), "--source-checkout", str(args.source_checkout), "--evidence", str(args.evidence), "--profile", args.profile]
        result = run_bounded(ProcessSpec.seconds(command, Path.cwd(), 1800, args.evidence, "offline-native-namespace"))
        print(result.stdout.read_text(encoding="utf-8"), end="")
        print(f"OFFLINE_NATIVE namespace closure={result.status} exit={result.exit_code}; evidence={args.evidence}")
        return 0 if result.success else (result.exit_code if result.exit_code not in (None, 0) else 1)
    finally:
        os.umask(previous_mask)


if __name__ == "__main__":
    raise SystemExit(main())
