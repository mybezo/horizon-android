import base64
from pathlib import Path
import stat
import subprocess
import sys
import tempfile
import unittest
from unittest import mock
sys.path.insert(0,str(Path(__file__).resolve().parents[1]/'tools'))
import android_dev_signing as signing
class SigningTests(unittest.TestCase):
    def test_missing_secret_generates_then_validates_a_private_development_key(self):
        with tempfile.TemporaryDirectory() as folder:
            path=Path(folder)/'keys/debug.keystore'
            def execute(args,**kwargs):
                if '-genkeypair' in args:path.write_bytes(b'test-generated-key')
            with mock.patch.object(signing.subprocess,'run',side_effect=execute) as run:
                self.assertEqual(signing.prepare(path,'keytool'),'generated')
            self.assertEqual(len(run.call_args_list),2)
            self.assertIn('-list',run.call_args_list[1].args[0])
            self.assertEqual(stat.S_IMODE(path.stat().st_mode),0o600)
    def test_valid_cache_is_reused_without_rotating_the_key(self):
        with tempfile.TemporaryDirectory() as folder:
            path=Path(folder)/'key';path.write_bytes(b'cached')
            with mock.patch.object(signing.subprocess,'run') as run:
                self.assertEqual(signing.prepare(path,'keytool'),'cache')
            self.assertEqual(path.read_bytes(),b'cached')
            self.assertEqual(run.call_count,1)
            self.assertIn('-list',run.call_args.args[0])
    def test_configured_secret_has_precedence(self):
        with tempfile.TemporaryDirectory() as folder:
            path=Path(folder)/'key';path.write_bytes(b'cached')
            with mock.patch.object(signing.subprocess,'run'):
                self.assertEqual(signing.prepare(path,'keytool',base64.b64encode(b'configured').decode()),'secret')
            self.assertEqual(path.read_bytes(),b'configured')
    def test_invalid_base64_does_not_overwrite_an_existing_key(self):
        with tempfile.TemporaryDirectory() as folder:
            path=Path(folder)/'key';path.write_bytes(b'cached')
            with self.assertRaises(ValueError):signing.prepare(path,'keytool','not base64!')
            self.assertEqual(path.read_bytes(),b'cached')
    def test_invalid_cached_key_fails_without_generating_another_identity(self):
        with tempfile.TemporaryDirectory() as folder:
            path=Path(folder)/'key';path.write_bytes(b'bad')
            with mock.patch.object(signing.subprocess,'run',side_effect=subprocess.CalledProcessError(1,['keytool'])) as run:
                with self.assertRaises(subprocess.CalledProcessError):signing.prepare(path,'keytool')
            self.assertEqual(run.call_count,1)
            self.assertEqual(path.read_bytes(),b'bad')
if __name__=='__main__':unittest.main()
