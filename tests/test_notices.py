"""Integration checks for locked-crate notices; /bin/true is never executed."""
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location('notices', Path(__file__).parents[1] / 'scripts/generate-notices.py')
notices = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(notices)
ROOT = Path(__file__).parents[1]


class NoticesTests(unittest.TestCase):
    def test_target_mapping_and_original_texts(self):
        rust_version = subprocess.check_output(['rustc', '--version'], text=True).split()[1]
        for target in ('x86_64-unknown-linux-gnu', 'aarch64-unknown-linux-gnu'):
            with self.subTest(target=target), tempfile.TemporaryDirectory() as tmp:
                out = Path(tmp) / 'notices'
                result = notices.collect(ROOT / 'Cargo.toml', target, Path('/bin/true'), out, rust_version)
                components = {c['name']: c for c in result['components']}
                self.assertEqual(result['target'], target)
                self.assertNotIn('windows-sys', components)
                self.assertNotIn('anstyle-wincon', components)
                self.assertIn('linux-static-hidraw', components['hidapi']['features'])
                self.assertIn('build/proc-macro', components['clap_derive']['role'])
                self.assertIn('runtime dependency', components['clap_builder']['role'])
                self.assertEqual(result['binary_sha256'], notices.digest(Path('/bin/true').read_bytes()))
                clap_text = (out / 'crates' / ('clap_builder-' + components['clap_builder']['version']) / 'LICENSE-MIT').read_text()
                self.assertIn('Copyright', clap_text)
                hid = components['hidapi']
                self.assertTrue(any('LICENSE-bsd.txt' in n['path'] for n in hid['notices']))
                self.assertTrue((out / 'rust/COPYRIGHT-library.html').is_file())
                self.assertTrue((out / 'rust/licenses/LLVM-exception.txt').is_file())
                for component in components.values():
                    for text in component['notices']:
                        self.assertEqual(notices.digest((out / text['path']).read_bytes()), text['sha256'])
                self.assertEqual(json.loads((out / 'components.json').read_text()), result)
                with self.assertRaisesRegex(ValueError, 'already exists'):
                    notices.collect(ROOT / 'Cargo.toml', target, Path('/bin/true'), out, rust_version)

    def test_corrupted_crate_is_rejected(self):
        rust_version = subprocess.check_output(['rustc', '--version'], text=True).split()[1]
        with tempfile.TemporaryDirectory() as tmp:
            with patch.object(notices, 'digest', return_value='0' * 64):
                with self.assertRaisesRegex(ValueError, 'Crate checksum mismatch'):
                    notices.collect(ROOT / 'Cargo.toml', 'x86_64-unknown-linux-gnu', Path('/bin/true'), Path(tmp) / 'notices', rust_version)

    def test_wrong_rust_version_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            with self.assertRaisesRegex(ValueError, 'toolchain does not match'):
                notices.collect(ROOT / 'Cargo.toml', 'x86_64-unknown-linux-gnu', Path('/bin/true'), Path(tmp) / 'notices', '0.0.0')


if __name__ == '__main__':
    unittest.main()
