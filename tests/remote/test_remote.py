"""Remote shell entry against real release executable and boundary fixtures."""
import hashlib
import io
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import tarfile
import tempfile
import unittest
import tomllib

ROOT = Path(__file__).resolve().parents[2]
BINARY = Path(os.environ.get('RELEASE_TEST_BINARY', ROOT / 'target/release/herdr-quota')).resolve()
TARGET = 'aarch64-apple-darwin' if platform.system() == 'Darwin' else 'x86_64-unknown-linux-musl'
VERSION = subprocess.check_output([str(BINARY), '--version'], text=True).strip().split()[1]

class RemoteLifecycle(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix='hq-remote-')
        self.addCleanup(self.tmp.cleanup)
        self.home = Path(self.tmp.name)
        self.bin = self.home / 'tools'
        self.bin.mkdir()
        if shutil.which('jq'):
            (self.bin / 'jq').symlink_to(shutil.which('jq'))
        self.assets = self.home / 'assets'
        self.assets.mkdir()
        self.package()
        self.script('codex', '''#!/bin/sh
while IFS= read -r line; do
case "$line" in
*'"method":"initialize"'*) echo '{"id":1,"result":{}}';;
*'"method":"account/read"'*) echo '{"id":2,"result":{"account":{"type":"chatgpt","email":"test","planType":"plus"}}}';;
*'"method":"account/rateLimits/read"'*) echo '{"id":3,"result":{"rateLimits":{"primary":{"windowDurationMins":300,"usedPercent":28,"resetsAt":4102444800},"secondary":{"windowDurationMins":10080,"usedPercent":67,"resetsAt":4102531200}}}}';;
esac
done
''')
        self.script('herdr', '#!/bin/sh\n[ ! -f "$HOME/stopped" ]\n')
        self.script('curl', '#!' + sys.executable + '\n' + '''
import json, os, pathlib, sys
args = sys.argv[1:]
home = pathlib.Path(os.environ['HOME'])
url = args[-1]
auth = '-K' in args
if auth:
    credentials = pathlib.Path(args[args.index('-K') + 1]).read_text()
    assert os.environ.get('GH_TOKEN') or os.environ.get('GITHUB_TOKEN')
    assert ('Bearer ' + (os.environ.get('GH_TOKEN') or os.environ['GITHUB_TOKEN'])) in credentials
    assert not any('secret_token' in a or 'preferred_token' in a for a in args)
with (home / 'requests').open('a') as log:
    log.write(json.dumps({'url':url, 'auth':auth}) + '\\n')
if (home / 'network-failure').exists(): sys.exit(22)
output = pathlib.Path(args[args.index('-o') + 1])
assets = home / 'assets'
if url.endswith('/releases/latest') or '/releases/tags/' in url:
    output.write_bytes((assets / 'release.json').read_bytes())
elif '/releases/assets/' in url:
    ident = url.rsplit('/',1)[1]
    if ident == '1': name = 'SHA256SUMS'
    else: name = next(p.name for p in assets.glob('*.tar.gz'))
    if (home / 'redirect').exists():
        pathlib.Path(args[args.index('-D') + 1]).write_text('HTTP/1.1 302 Found\\r\\nLocation: https://release-assets.githubusercontent.com/' + name + '\\r\\n\\r\\n')
        output.write_bytes(b'')
        print('302', end=''); sys.exit()
    output.write_bytes((assets / name).read_bytes())
elif 'SHA256SUMS' in url:
    output.write_bytes((assets / 'SHA256SUMS').read_bytes())
else:
    output.write_bytes((assets / url.rsplit('/',1)[1]).read_bytes())
if '-w' in args: print('200', end='')
''')
        self.env = {**os.environ, 'HOME': str(self.home), 'TMPDIR': str(self.home), 'PATH': f'{self.bin}:/usr/bin:/bin', 'XDG_DATA_HOME': str(self.home / 'data'), 'XDG_CACHE_HOME': str(self.home / 'cache'), 'XDG_CONFIG_HOME': str(self.home / 'config'), 'GH_TOKEN':'', 'GITHUB_TOKEN':''}
        self.config = self.home / 'config.toml'
        self.original = '# preserved\n[ui]\ntab_bar_position = "bottom"\nhide_tab_bar_when_single_tab = true\ntab_bar_right = [{type="hostname"}]\n'
        self.config.write_text(self.original)
    def script(self, name, text):
        p = self.bin / name
        p.write_text(text)
        p.chmod(0o700)
    def package(self, extra=None, binary=BINARY, version=VERSION):
        for path in self.assets.glob('*.tar.gz'):
            path.unlink()
        name = f'herdr-quota-v{version}-{TARGET}.tar.gz'
        archive = self.assets / name
        if extra:
            with tarfile.open(archive, 'w:gz') as t:
                t.add(binary, arcname='herdr-quota', recursive=False)
                info = tarfile.TarInfo(extra)
                info.size = 3
                t.addfile(info, io.BytesIO(b'bad'))
        else:
            subprocess.run([sys.executable, str(ROOT / 'scripts/release.py'), 'package', '--binary', str(binary), '--tag', 'v' + version, '--target', TARGET, '--output', str(self.assets)], check=True, capture_output=True)
        (self.assets / 'SHA256SUMS').write_text(f'{hashlib.sha256(archive.read_bytes()).hexdigest()}  {name}\n')
        (self.assets / 'release.json').write_text(json.dumps({'tag_name':f'v{version}', 'assets':[{'id':1,'name':'SHA256SUMS','state':'uploaded'}, {'id':2,'name':name,'state':'uploaded'}]}))
    def run_remote(self, action='install', success=True):
        r = subprocess.run(['/bin/sh', str(ROOT / 'scripts/remote.sh'), action, '--config', str(self.config), '--install-dir', str(self.home / 'installed bin'), '--codex', str(self.bin / 'codex'), '--herdr', str(self.bin / 'herdr')], env=self.env, capture_output=True, text=True, timeout=30)
        self.assertEqual(r.returncode == 0, success, r.stdout + r.stderr)
        self.assertNotIn('secret_token', r.stdout + r.stderr)
        self.assertNotIn('preferred_token', r.stdout + r.stderr)
        self.assertEqual(list(self.home.glob('herdr-quota.*')), [])
        return r
    @property
    def installed(self):
        return self.home / 'installed bin/herdr-quota'
    def test_native_install_repeat_update_retry_and_uninstall(self):
        self.run_remote()
        self.assertEqual(self.installed.read_bytes(), BINARY.read_bytes())
        first = self.config.read_text()
        self.assertIn('tab_bar_position = "bottom"', first)
        self.assertIn('type="hostname"', first)
        self.assertEqual(first.count('# herdr-quota'), 1)
        subprocess.run([str(self.installed), 'refresh', '--codex', str(self.bin / 'codex')], env=self.env, check=True, capture_output=True)
        entries = tomllib.loads(first)['ui']['tab_bar_right']
        command = next(e['command'] for e in entries if e.get('type') == 'command')
        status = subprocess.run(['/bin/sh', '-c', command], env=self.env, capture_output=True, text=True, timeout=20)
        self.assertEqual(status.returncode, 0, status.stderr)
        self.assertIn('72%', status.stdout)
        self.assertIn('33%', status.stdout)
        self.run_remote()
        self.assertEqual(self.config.read_text(), first)
        # Replace an older/corrupt installed copy; staged native installer must replace it.
        self.installed.write_bytes(b'previous installation')
        self.run_remote()
        old = self.installed.read_bytes()
        (self.home / 'network-failure').touch()
        self.run_remote(success=False)
        self.assertEqual(self.installed.read_bytes(), old)
        (self.home / 'network-failure').unlink()
        (self.assets / 'SHA256SUMS').write_text('0' * 64 + '  ' + next(self.assets.glob('*.tar.gz')).name + '\n')
        self.run_remote(success=False)
        self.assertEqual(self.installed.read_bytes(), old)
        self.package()
        self.run_remote()
        (self.home / 'stopped').touch()
        self.run_remote('uninstall', success=False)
        self.assertTrue(self.installed.exists())
        (self.home / 'stopped').unlink()
        self.installed.unlink()  # Missing binary still permits remote cleanup.
        (self.bin / 'codex').unlink()
        self.run_remote('uninstall')
        self.run_remote('uninstall')
        self.assertFalse(self.installed.exists())
        self.assertNotIn('# herdr-quota', self.config.read_text())
        self.assertIn('hide_tab_bar_when_single_tab = true', self.config.read_text())

    def test_native_previous_release_updates_to_current_version(self):
        previous = Path(os.environ['PREVIOUS_TEST_BINARY']).resolve()
        self.package(binary=previous, version='0.0.0')
        self.run_remote()
        self.assertEqual(subprocess.check_output([str(self.installed), '--version'], text=True).strip(), 'herdr-quota 0.0.0')
        first = self.config.read_text()
        self.package()
        self.run_remote()
        self.assertEqual(subprocess.check_output([str(self.installed), '--version'], text=True).strip(), 'herdr-quota ' + VERSION)
        self.assertEqual(first, self.config.read_text())

    def test_private_api_downloads_and_redirects_keep_credentials_on_api(self):
        self.env['GITHUB_TOKEN'] = 'secret_token'
        self.run_remote()
        requests = [json.loads(line) for line in (self.home / 'requests').read_text().splitlines()]
        self.assertTrue(all(r['auth'] for r in requests))
        (self.home / 'requests').unlink()
        (self.home / 'redirect').touch()
        self.env['GH_TOKEN'] = 'preferred_token'
        self.run_remote()
        requests = [json.loads(line) for line in (self.home / 'requests').read_text().splitlines()]
        cloud = [r for r in requests if 'release-assets.githubusercontent.com' in r['url']]
        self.assertEqual(len(cloud), 2)
        self.assertTrue(all(not r['auth'] for r in cloud))
        (self.home / 'network-failure').touch()
        self.run_remote(success=False)

    def test_unsafe_archive_and_wrong_version_preserve_installed_program(self):
        self.run_remote()
        old = self.installed.read_bytes()
        for name in ('../outside', 'extra-file'):
            self.package(extra=name)
            self.run_remote(success=False)
            self.assertEqual(self.installed.read_bytes(), old)
            self.assertFalse((self.home / 'outside').exists())
        self.package()
        archive = next(self.assets.glob('*.tar.gz'))
        with tarfile.open(archive, 'w:gz') as t:
            info = tarfile.TarInfo('herdr-quota')
            info.type = tarfile.SYMTYPE
            info.linkname = '/bin/sh'
            t.addfile(info)
        (self.assets / 'SHA256SUMS').write_text(hashlib.sha256(archive.read_bytes()).hexdigest() + '  ' + archive.name + '\n')
        self.run_remote(success=False)
        self.assertEqual(self.installed.read_bytes(), old)
        self.package()
        archive = next(self.assets.glob('*.tar.gz'))
        name = archive.name.replace('v' + VERSION, 'v99.0.0')
        archive.rename(self.assets / name)
        (self.assets / 'SHA256SUMS').write_text(hashlib.sha256((self.assets / name).read_bytes()).hexdigest() + '  ' + name + '\n')
        self.run_remote(success=False)
        self.assertEqual(self.installed.read_bytes(), old)

    def test_unsupported_platform_and_duplicate_manifest_fail_before_install(self):
        self.script('uname', '#!/bin/sh\necho unsupported\n')
        self.run_remote(success=False)
        self.assertFalse(self.installed.exists())
        (self.bin / 'uname').unlink()
        sums = self.assets / 'SHA256SUMS'
        sums.write_text(sums.read_text() * 2)
        self.run_remote(success=False)
        self.assertFalse(self.installed.exists())

if __name__ == '__main__': unittest.main()
