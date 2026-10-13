"""Download the official development archives pinned in core/resources/official.json."""

from __future__ import annotations

import hashlib
import json
import os
import shutil
import ssl
import sys
import urllib.error
import urllib.request
from pathlib import Path

from autosar_tooling.config import ROOT, archive_path

# www.autosar.org sends only its leaf certificate. Its issuer is fetched from the
# leaf's AIA address and trusted only with this fingerprint, beside the system
# CA store; verification is never disabled. Update it only after reviewing a
# changed site certificate chain.
INTERMEDIATE_URL = "http://certificates.starfieldtech.com/repository/sfig2.crt"
INTERMEDIATE_SHA256 = "93a07898d89b2cca166ba6f1f8a14138ce43828e491b831926bc8247d391cc72"
TIMEOUT = 60


class FetchError(Exception):
    pass


def tls_context() -> ssl.SSLContext:
    try:
        with urllib.request.urlopen(INTERMEDIATE_URL, timeout=TIMEOUT) as response:
            certificate = response.read()
    except (urllib.error.URLError, OSError) as error:
        raise FetchError(f"cannot download the intermediate certificate {INTERMEDIATE_URL}: {error}") from error
    fingerprint = hashlib.sha256(certificate).hexdigest()
    if fingerprint != INTERMEDIATE_SHA256:
        raise FetchError(
            f"intermediate certificate fingerprint {fingerprint} differs from the pinned {INTERMEDIATE_SHA256}; "
            "review the site certificate chain before updating the pin"
        )
    context = ssl.create_default_context()
    context.load_verify_locations(cadata=ssl.DER_cert_to_PEM_cert(certificate))
    return context


def file_digest(path: Path) -> str:
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def fetch(root: Path = ROOT) -> int:
    """Download missing archives; every archive, cached or new, must match its pinned SHA-256."""
    records = json.loads((root / "core/resources/official.json").read_text(encoding="utf-8"))
    context = None
    try:
        for item in records:
            name, expected = item["name"], item["sha256"]
            if not expected:
                raise FetchError(f"{name}: no pinned SHA-256 in core/resources/official.json")
            target = archive_path(item["environment"], item["path"], root)
            if not target.exists():
                context = context or tls_context()
                target.parent.mkdir(parents=True, exist_ok=True)
                partial = target.with_name(target.name + ".part")
                try:
                    with urllib.request.urlopen(item["url"], timeout=TIMEOUT, context=context) as response, partial.open("wb") as output:
                        shutil.copyfileobj(response, output, 1 << 20)
                    actual = file_digest(partial)
                    if actual != expected:
                        raise FetchError(f"{name}: SHA-256 mismatch for {item['url']}: downloaded {actual}, required {expected}")
                    os.replace(partial, target)
                except (urllib.error.URLError, OSError) as error:
                    raise FetchError(f"{name}: download failed for {item['url']}: {error}") from error
                finally:
                    partial.unlink(missing_ok=True)
                print(f"{name}: downloaded {target}; SHA-256 {expected}")
                continue
            # An existing file may be the developer's own copy; report it, never replace it.
            actual = file_digest(target)
            if actual != expected:
                raise FetchError(f"{name}: SHA-256 mismatch for existing {target}: found {actual}, required {expected}")
            print(f"{name}: verified {target}; SHA-256 {expected}")
    except FetchError as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    return 0
