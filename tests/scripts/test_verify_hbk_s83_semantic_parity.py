from __future__ import annotations

import importlib.util
import struct
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "verify-hbk-s83-semantic-parity.py"
SPEC = importlib.util.spec_from_file_location("verify_hbk_s83_semantic_parity", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def flat_header(
    magic: bytes, layout_version: int, layout_flags: int, section_count: int
) -> bytes:
    header = bytearray(216)
    header[:8] = magic
    struct.pack_into("<I", header, 8, layout_version)
    struct.pack_into("<I", header, 12, MODULE.EXTRACTION_SCHEMA_VERSION)
    struct.pack_into("<I", header, 16, MODULE.PROVIDER_SCHEMA_VERSION)
    struct.pack_into("<I", header, 20, layout_flags)
    struct.pack_into("<I", header, 24, section_count)
    struct.pack_into("<Q", header, 48, MODULE.HBK_SIZE)
    struct.pack_into("<Q", header, 56, MODULE.PROVIDER_SIZE)
    header[64:128] = MODULE.HBK_SHA256.encode("ascii")
    header[128:192] = MODULE.PROVIDER_SHA256.encode("ascii")
    header[192:194] = b"ru"
    platform = MODULE.PLATFORM_VERSION.encode("ascii")
    header[200 : 200 + len(platform)] = platform
    return bytes(header)


class S83SemanticParityHeaderTests(unittest.TestCase):
    def test_recognizes_every_predeclared_flat_identity(self) -> None:
        identities = (
            ("flat-h2", b"HBKFH2\0\0", 2, 0, 63),
            ("flat-l1", b"HBKFH2\0\0", 3, 1, 63),
            ("flat-i1", b"HBKFI1\0\0", 3, 1, 64),
            ("flat-r1", b"HBKFR1\0\0", 1, 1, 71),
        )
        with tempfile.TemporaryDirectory() as tmp:
            directory = Path(tmp)
            for kind, magic, layout, flags, sections in identities:
                with self.subTest(kind=kind):
                    path = directory / kind
                    path.write_bytes(flat_header(magic, layout, flags, sections))
                    metadata = MODULE.artifact_header_metadata(path)
                    self.assertEqual(metadata["kind"], kind)
                    self.assertEqual(metadata["layout_version"], layout)
                    self.assertEqual(metadata["layout_flags"], flags)
                    self.assertEqual(metadata["section_count"], sections)

    def test_rejects_unregistered_flat_identity(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "unknown-flat"
            path.write_bytes(flat_header(b"HBKFH2\0\0", 4, 1, 63))
            with self.assertRaisesRegex(RuntimeError, "unsupported flat artifact identity"):
                MODULE.artifact_header_metadata(path)

    def test_flat_h2_contract_allows_only_formation_compatible_backends(self) -> None:
        self.assertEqual(
            MODULE.ARTIFACT_CONTRACTS["flat-h2"]["backends"],
            (
                "s83-f0-semantic",
                "s83-d1-semantic",
                "s83-p1-semantic",
            ),
        )


if __name__ == "__main__":
    unittest.main()
