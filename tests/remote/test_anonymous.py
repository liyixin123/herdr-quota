"""Anonymous public-download regression against the real GitHub endpoints.

Cleared of GH_TOKEN/GITHUB_TOKEN, this suite mirrors the README one-command
path: fetch the pinned bootstrap from raw.githubusercontent.com, then exercise
the real releases/latest SHA256SUMS and archive, checksum verification,
install/update/uninstall, failure protection, and retry.
"""
import hashlib
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import tempfile
import tomllib
import unittest
import urllib.request

# Bootstrap pin shared with the README one-liner; re-pin both when cutting a release.
BOOTSTRAP_REF = 'v0.1.1'
SCRIPT_URL = f'https://raw.githubusercontent.com/liyixin123/herdr-quota/{BOOTSTRAP_REF}/scripts/remote.sh'
LATEST_SUMS_URL = 'https://github.com/liyixin123/herdr-quota/releases/latest/download/SHA256SUMS'
TARGET = 'aarch64-apple-darwin' if platform.system() == 'Darwin' else 'x86_64-unknown-linux-musl'
REAL_CURL = shutil.which('curl')


def fetch(url):
    with urllib.request.urlopen(url, timeout=120) as response:
        return response.read()


class AnonymousPublicDownload(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix='hq-anonymous-')
        self.addCleanup(self.tmp.cleanup)
        self.home = Path(self.tmp.name)
        self.tools = self.home / 'tools'
        self.tools.mkdir()
        for name, text in {
            'herdr': '#!/bin/sh\nexit 0\n',
            'codex': '''#!/bin/sh
while IFS= read -r line; do
case "$line" in
*'"method":"initialize"'*) echo '{"id":1,"result":{}}';;
*'"method":"account/read"'*) echo '{"id":2,"result":{"account":{"type":"chatgpt","email":"fixture","planType":"plus"}}}';;
*'"method":"account/rateLimits/read"'*) echo '{"id":3,"result":{"rateLimits":{"primary":{"windowDurationMins":300,"usedPercent":28,"resetsAt":4102444800},"secondary":{"windowDurationMins":10080,"usedPercent":67,"resetsAt":4102531200}}}}';;
esac
done
''',
        }.items():
            path = self.tools / name
            path.write_text(text)
            path.chmod(0o700)
        self.env = {**os.environ, 'HOME': str(self.home), 'TMPDIR': str(self.home), 'PATH': str(self.tools) + ':/usr/bin:/bin', 'XDG_CONFIG_HOME': str(self.home / 'config'), 'XDG_DATA_HOME': str(self.home / 'data'), 'XDG_CACHE_HOME': str(self.home / 'cache')}
        for credential in ('GH_TOKEN', 'GITHUB_TOKEN'):
            self.env.pop(credential, None)
        for pinned in ('HERDR_QUOTA_TAG', 'HERDR_QUOTA_BOOTSTRAP_REF', 'HERDR_CONFIG_PATH'):
            self.env.pop(pinned, None)
        self.config = self.home / 'config/herdr/config.toml'
        self.config.parent.mkdir(parents=True)
        self.config.write_text('# preserved\n[ui]\ntab_bar_position="bottom"\nhide_tab_bar_when_single_tab = true\ntab_bar_right=[{type="hostname"}]\n')
        self.binary = self.home / '.local/bin/herdr-quota'
        self.script = self.home / 'remote.sh'
        self.script.write_bytes(fetch(SCRIPT_URL))
        self.script.chmod(0o700)

    def published_release(self):
        entries = {}
        for line in fetch(LATEST_SUMS_URL).decode().splitlines():
            digest, name = line.split()
            self.assertRegex(digest, r'^[0-9a-f]{64}$')
            entries[name] = digest
        versions = set()
        targets = set()
        for name in entries:
            match = re.fullmatch(r'herdr-quota-v(.+)-(aarch64-apple-darwin|x86_64-unknown-linux-musl)\.tar\.gz', name)
            self.assertIsNotNone(match, name)
            versions.add(match.group(1))
            targets.add(match.group(2))
        self.assertEqual(len(versions), 1)
        self.assertEqual(targets, {'aarch64-apple-darwin', 'x86_64-unknown-linux-musl'})
        version = versions.pop()
        archive = f'herdr-quota-v{version}-{TARGET}.tar.gz'
        return version, archive, entries[archive]

    def run_remote(self, action='install', success=True):
        result = subprocess.run(['/bin/sh', str(self.script), action], env=self.env, capture_output=True, text=True, timeout=300)
        self.assertEqual(result.returncode == 0, success, result.stdout + result.stderr)
        self.assertEqual(list(self.home.glob('herdr-quota.*')), [])
        return result

    def stub_curl(self, text):
        path = self.tools / 'curl'
        path.write_text('#!/bin/sh\n' + text)
        path.chmod(0o700)

    def installed_version(self):
        return subprocess.check_output([str(self.binary), '--version'], env=self.env, text=True).strip()

    def test_first_install_repeat_install_and_quota_display(self):
        version, archive, digest = self.published_release()
        package = fetch(f'https://github.com/liyixin123/herdr-quota/releases/download/v{version}/{archive}')
        self.assertEqual(hashlib.sha256(package).hexdigest(), digest)
        self.run_remote()
        self.assertEqual(self.installed_version(), 'herdr-quota ' + version)
        first = self.config.read_text()
        ui = tomllib.loads(first)['ui']
        self.assertEqual(ui['tab_bar_position'], 'bottom')
        self.assertIs(ui['hide_tab_bar_when_single_tab'], False)
        self.assertEqual(sum('# herdr-quota' in entry.get('command', '') for entry in ui['tab_bar_right']), 1)
        self.assertTrue(any(entry.get('type') == 'hostname' for entry in ui['tab_bar_right']))
        self.run_remote()
        self.assertEqual(self.config.read_text(), first)
        subprocess.run([str(self.binary), 'refresh', '--codex', str(self.tools / 'codex')], env=self.env, check=True, capture_output=True, timeout=120)
        command = next(entry['command'] for entry in ui['tab_bar_right'] if entry.get('type') == 'command')
        status = subprocess.run(['/bin/sh', '-c', command], env=self.env, capture_output=True, text=True, timeout=60)
        self.assertEqual(status.returncode, 0, status.stderr)
        self.assertIn('72%', status.stdout)
        self.assertIn('33%', status.stdout)

    def test_update_from_previous_version(self):
        previous = Path(os.environ['PREVIOUS_TEST_BINARY']).resolve()
        subprocess.run([str(previous), 'install'], env=self.env, check=True, capture_output=True, timeout=120)
        self.assertEqual(self.installed_version(), 'herdr-quota 0.0.0')
        first = self.config.read_text()
        version, _, _ = self.published_release()
        self.run_remote()
        self.assertEqual(self.installed_version(), 'herdr-quota ' + version)
        self.assertEqual(self.config.read_text(), first)

    def test_download_and_checksum_failure_preserve_install_then_retry(self):
        self.run_remote()
        old = self.binary.read_bytes()
        first = self.config.read_text()
        self.stub_curl('exit 22\n')
        self.run_remote(success=False)
        self.assertEqual(self.binary.read_bytes(), old)
        self.assertEqual(self.config.read_text(), first)
        self.stub_curl(f'''{REAL_CURL} "$@" || exit $?
for last in "$@"; do :; done
case "$last" in
*.tar.gz) ;;
*) exit 0 ;;
esac
out=""
prev=""
for arg in "$@"; do
    [ "$prev" = "-o" ] && out="$arg"
    prev="$arg"
done
[ -n "$out" ] && printf 'X' >> "$out"
exit 0
''')
        result = self.run_remote(success=False)
        self.assertIn('SHA256', result.stderr)
        self.assertEqual(self.binary.read_bytes(), old)
        self.assertEqual(self.config.read_text(), first)
        (self.tools / 'curl').unlink()
        self.run_remote()
        version, _, _ = self.published_release()
        self.assertEqual(self.installed_version(), 'herdr-quota ' + version)

    def test_uninstall_after_manual_removal_and_repeated_uninstall(self):
        self.run_remote()
        self.binary.unlink()
        (self.tools / 'codex').unlink()
        self.run_remote('uninstall')
        self.assertFalse(self.binary.exists())
        final = self.config.read_text()
        self.assertNotIn('# herdr-quota', final)
        self.assertIn('# preserved', final)
        self.assertIn('tab_bar_position="bottom"', final)
        self.assertIn('hide_tab_bar_when_single_tab = true', final)
        self.assertIn('type="hostname"', final)
        self.run_remote('uninstall')
        self.assertEqual(self.config.read_text(), final)


if __name__ == '__main__': unittest.main()
