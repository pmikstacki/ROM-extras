import importlib.util
import json
import tempfile
from pathlib import Path
import unittest
spec = importlib.util.spec_from_file_location('qualify', Path(__file__).with_name('qualify.py'))
qualify = importlib.util.module_from_spec(spec)
spec.loader.exec_module(qualify)

class PrivateEvidenceTests(unittest.TestCase):
    def test_exclusive_creation_has_private_permissions(self):
        with tempfile.TemporaryDirectory() as root:
            path = Path(root)/'evidence.json'
            qualify.private_json(path, {'marker':'fixture-only'})
            self.assertEqual(path.stat().st_mode & 0o777, 0o600)
            self.assertEqual(json.loads(path.read_text()), {'marker':'fixture-only'})
            with self.assertRaises(FileExistsError):
                qualify.private_json(path, {'marker':'overwritten'})
            self.assertEqual(json.loads(path.read_text()), {'marker':'fixture-only'})

if __name__ == '__main__':
    unittest.main()
