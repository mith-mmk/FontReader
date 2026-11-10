#!/usr/bin/env python3
"""Verify the pinned Unicode 17.0.0 grapheme fixture without network I/O."""

import hashlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "data" / "unicode" / "17.0.0" / "GraphemeBreakTest.txt"
LICENSE = ROOT / "data" / "unicode" / "LICENSE.txt"
SOURCE_SHA256 = "e2d134d2c52919bace503ebb6a551c1855fe1a1faec18478c78fff254a1793ec"
LICENSE_SHA256 = "e7a93b009565cfce55919a381437ac4db883e9da2126fa28b91d12732bc53d96"


def check():
    raw = SOURCE.read_bytes()
    if hashlib.sha256(raw).hexdigest() != SOURCE_SHA256:
        raise ValueError("Pinned grapheme test SHA-256 does not match")
    if hashlib.sha256(LICENSE.read_bytes()).hexdigest() != LICENSE_SHA256:
        raise ValueError("Pinned Unicode license SHA-256 does not match")
    source = raw.decode("utf-8")
    if not source.startswith("# GraphemeBreakTest-17.0.0.txt\n"):
        raise ValueError("Unexpected Unicode grapheme version")
    cases = [line.split("#", 1)[0].strip() for line in source.splitlines()]
    if sum(bool(case) for case in cases) != 766:
        raise ValueError("Expected all 766 official grapheme test cases")
    print("Unicode 17.0.0 grapheme fixture and Unicode license verified")


if __name__ == "__main__":
    check()
