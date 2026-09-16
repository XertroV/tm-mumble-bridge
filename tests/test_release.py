"""Release gates and archive contract, with no network or Rust dependency."""
import importlib.util
from pathlib import Path
import tarfile
import tempfile
import unittest
import zipfile

spec = importlib.util.spec_from_file_location('release', Path(__file__).parents[1] / 'scripts/release.py')
release = importlib.util.module_from_spec(spec)
spec.loader.exec_module(release)


class ReleaseTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        (self.root / 'Cargo.toml').write_text('[package]\nname="tm-mumble-link"\nversion="1.2.3"\n')
        (self.root / 'Cargo.lock').write_text('[[package]]\nname="tm-mumble-link"\nversion="1.2.3"\n')
        for name in release.DOCS:
            (self.root / name).write_text(name)
        self.binary = self.root / 'binary'
        self.binary.write_bytes(b'binary fixture')

    def test_metadata_checks_tag_lock_and_notes(self):
        self.assertEqual(release.metadata(self.root, 'v1.2.3')[0], '1.2.3')
        with self.assertRaises(ValueError):
            release.metadata(self.root, 'v1.2.4')
        with self.assertRaises(FileNotFoundError):
            release.metadata(self.root, require_notes=True)
        (self.root / 'releases').mkdir()
        notes = self.root / 'releases/v1.2.3.md'
        notes.write_text('# v1.2.3\n')
        with self.assertRaises(ValueError):
            release.metadata(self.root, require_notes=True)
        notes.write_text('# v1.2.3\n\nAll changes.\n')
        release.metadata(self.root, require_notes=True)
        (self.root / 'Cargo.lock').write_text('[[package]]\nname="tm-mumble-link"\nversion="1.2.2"\n')
        with self.assertRaises(ValueError):
            release.metadata(self.root)

    def test_packages_include_executable_docs_and_valid_checksums(self):
        linux = release.package(self.root, self.binary, 'linux-x86_64')
        with tarfile.open(linux) as bundle:
            self.assertEqual(set(bundle.getnames()), {'tm-mumble-link', *release.DOCS})
            self.assertEqual(bundle.getmember('tm-mumble-link').mode, 0o755)
            self.assertEqual(bundle.extractfile('tm-mumble-link').read(), b'binary fixture')
        with self.assertRaises(ValueError):
            release.verify_assets(self.root / 'dist', '1.2.3')
        windows = release.package(self.root, self.binary, 'windows-x86_64')
        with zipfile.ZipFile(windows) as bundle:
            self.assertEqual(set(bundle.namelist()), {'tm-mumble-link.exe', *release.DOCS})
        self.assertEqual(len(release.verify_assets(self.root / 'dist', '1.2.3')), 4)
        linux.write_bytes(b'corrupted')
        with self.assertRaises(ValueError):
            release.verify_assets(self.root / 'dist', '1.2.3')

    def test_missing_binary_is_rejected(self):
        self.binary.unlink()
        with self.assertRaises(ValueError):
            release.package(self.root, self.binary, 'linux-x86_64')


if __name__ == '__main__':
    unittest.main()
