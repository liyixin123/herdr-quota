"""Exercise packaged executables and release downloads through their public CLIs."""
import hashlib
import http.server
import os
import json
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import threading
import unittest

ROOT = Path(__file__).resolve().parents[2]
TARGET = 'aarch64-apple-darwin' if sys.platform == 'darwin' else 'x86_64-unknown-linux-musl'
BINARY = Path(os.environ.get('RELEASE_TEST_BINARY', ROOT / 'target/release/herdr-quota')).resolve()
TAG = os.environ.get('RELEASE_TAG') or 'v' + subprocess.check_output([str(BINARY), '--version'], text=True).strip().split()[1]

class ReleaseTest(unittest.TestCase):
    def cli(self, *args, ok=True):
        result = subprocess.run([sys.executable, str(ROOT / 'scripts/release.py'), *map(str, args)], capture_output=True, text=True)
        self.assertEqual(result.returncode == 0, ok, result.stdout + result.stderr)
        return result

    def test_package_is_downloadable_verified_and_runs_without_rust(self):
        with tempfile.TemporaryDirectory() as work:
            directory = Path(work)
            self.cli('package', '--binary', BINARY, '--target', TARGET, '--tag', TAG, '--output', directory)
            archive = directory / f'herdr-quota-{TAG}-{TARGET}.tar.gz'
            self.assertTrue(archive.is_file())
            digest = hashlib.sha256(archive.read_bytes()).hexdigest()
            self.assertEqual((directory / 'SHA256SUMS').read_text(), f'{digest}  {archive.name}\n')
            with tarfile.open(archive) as package:
                self.assertEqual(package.getnames(), ['herdr-quota'])
                (directory / 'unpacked').mkdir()
                executable = directory / 'unpacked/herdr-quota'
                executable.write_bytes(package.extractfile('herdr-quota').read())
                executable.chmod(0o755)
            subprocess.run([sys.executable, str(ROOT / 'tests/release/smoke.py'), directory / 'unpacked/herdr-quota', TAG], check=True)
            handler = lambda *args, **kwargs: http.server.SimpleHTTPRequestHandler(*args, directory=directory, **kwargs)
            with http.server.ThreadingHTTPServer(('127.0.0.1', 0), handler) as server:
                thread = threading.Thread(target=server.serve_forever, daemon=True)
                thread.start()
                try:
                    url = f'http://127.0.0.1:{server.server_port}'
                    self.cli('download', '--base-url', url, '--tag', TAG, '--target', TARGET, '--output', directory / 'download')
                    archive.write_bytes(b'corrupted download')
                    self.cli('download', '--base-url', url, '--tag', TAG, '--target', TARGET, '--output', directory / 'corrupt', ok=False)
                finally:
                    server.shutdown()

    def test_published_release_requires_both_downloads_and_matching_checksums(self):
        with tempfile.TemporaryDirectory() as work:
            directory = Path(work)
            names = [f'herdr-quota-{TAG}-{target}.tar.gz' for target in ('aarch64-apple-darwin', 'x86_64-unknown-linux-musl')]
            data = [b'macos-package', b'linux-package']
            manifest = ''.join(f'{hashlib.sha256(body).hexdigest()}  {name}\n' for name, body in zip(names, data)).encode()
            assets = []
            for index, (name, body) in enumerate(zip(names + ['SHA256SUMS'], data + [manifest])):
                (directory / str(index)).write_bytes(body)
                assets.append({'id': index, 'name': name, 'browser_download_url': f'https://github.com/liyixin123/herdr-quota/releases/download/{TAG}/{name}'})
            (directory / 'release.json').write_text(json.dumps({'tag_name': TAG, 'assets': assets}))
            gh = directory / 'gh'
            gh.write_text('#!' + sys.executable + '\nimport pathlib,sys\np=pathlib.Path(__file__).parent\nendpoint=sys.argv[2]\nname=endpoint.rsplit("/",1)[-1] if "/assets/" in endpoint else "release.json"\nsys.stdout.buffer.write((p/name).read_bytes())\n')
            gh.chmod(0o755)
            command = [sys.executable, str(ROOT / 'scripts/verify-published.py'), '--tag', TAG, '--output', str(directory / 'verified')]
            env = {**os.environ, 'PATH': str(directory)}
            result = subprocess.run(command, env=env, capture_output=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            (directory / '1').write_bytes(b'corrupt published package')
            result = subprocess.run(command, env=env, capture_output=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn(b'checksum mismatch', result.stderr)

if __name__ == '__main__':
    unittest.main()
