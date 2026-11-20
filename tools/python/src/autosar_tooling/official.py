"""Download the official development archives pinned in core/resources/official.json."""

from __future__ import annotations

import hashlib
import http.client
import io
import json
import os
import shutil
import socket
import ssl
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path
from typing import BinaryIO
from urllib.parse import urlsplit

from autosar_tooling.config import ROOT, archive_path

# www.autosar.org sends only its leaf certificate. Its issuer is fetched from the
# leaf's AIA address and trusted only with this fingerprint, beside the system
# CA store; verification is never disabled. Update it only after reviewing a
# changed site certificate chain.
INTERMEDIATE_URL = "http://certificates.starfieldtech.com/repository/sfig2.crt"
INTERMEDIATE_SHA256 = "93a07898d89b2cca166ba6f1f8a14138ce43828e491b831926bc8247d391cc72"
TIMEOUT = 60
ATTEMPTS = 3


class FetchError(Exception):
    pass


class RejectRedirects(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


def dns_diagnostic(url: str) -> str:
    host = urlsplit(url).hostname
    try:
        records = socket.getaddrinfo(host, None, type=socket.SOCK_STREAM)
    except OSError as error:
        return f"URL host {host} DNS lookup failed: {error}"
    addresses = ", ".join(dict.fromkeys(record[4][0] for record in records))
    return f"URL host {host} DNS addresses: {addresses}"


def download(url: str, output: BinaryIO, context: ssl.SSLContext | None = None) -> None:
    opener = urllib.request.build_opener(
        RejectRedirects(), urllib.request.HTTPSHandler(context=context)
    )
    for attempt in range(1, ATTEMPTS + 1):
        try:
            with opener.open(url, timeout=TIMEOUT) as response:
                output.seek(0)
                output.truncate()
                shutil.copyfileobj(response, output, 1 << 20)
        except (OSError, http.client.HTTPException) as error:
            reason = error.reason if isinstance(error, urllib.error.URLError) else error
            retryable = isinstance(reason, (ConnectionError, TimeoutError)) or (
                isinstance(reason, socket.gaierror) and reason.errno == socket.EAI_AGAIN
            )
            message = (
                f"download failed for {url} (attempt {attempt}/{ATTEMPTS}, "
                f"socket timeout {TIMEOUT}s): {type(reason).__name__}: {error}"
            )
            if isinstance(error, urllib.error.HTTPError):
                error.close()
            else:
                message += f"; {dns_diagnostic(url)}"
            if not retryable or attempt == ATTEMPTS:
                raise FetchError(message) from error
            print(f"warning: {message}; retrying", file=sys.stderr)
            time.sleep(attempt)
        else:
            return


def tls_context() -> ssl.SSLContext:
    output = io.BytesIO()
    download(INTERMEDIATE_URL, output)
    certificate = output.getvalue()
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
                    with partial.open("wb") as output:
                        download(item["url"], output, context)
                    actual = file_digest(partial)
                    if actual != expected:
                        raise FetchError(f"{name}: SHA-256 mismatch for {item['url']}: downloaded {actual}, required {expected}")
                    os.replace(partial, target)
                except OSError as error:
                    raise FetchError(f"{name}: cannot write archive {target}: {error}") from error
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
