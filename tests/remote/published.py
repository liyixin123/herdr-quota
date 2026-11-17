#!/usr/bin/env python3
"""After-publication acceptance using the real private bootstrap and release API."""
import os
import re
from urllib.parse import quote
from pathlib import Path
import subprocess
import tempfile
import tomllib

# This job intentionally does not build a binary. Both script and native manager
# come from GitHub. By default the bootstrap is pinned to the release tag;
# branch CI can verify a fixed bootstrap against an already published native asset.
tag = os.environ['HERDR_QUOTA_TAG']
bootstrap_ref = quote(os.environ.get('HERDR_QUOTA_BOOTSTRAP_REF', tag), safe='')
with tempfile.TemporaryDirectory(prefix='hq-published-') as work:
    home = Path(work)
    tools = home / 'tools'
    tools.mkdir()
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
'''
    }.items():
        path = tools / name
        path.write_text(text)
        path.chmod(0o700)
    env = {**os.environ, 'HOME': work, 'XDG_CONFIG_HOME': str(home / 'config'), 'XDG_DATA_HOME': str(home / 'data'), 'XDG_CACHE_HOME': str(home / 'cache'), 'PATH': str(tools) + ':' + os.environ['PATH']}
    env.pop('HERDR_CONFIG_PATH', None)
    config = home / 'config/herdr/config.toml'
    config.parent.mkdir(parents=True)
    config.write_text('# preserved\n[ui]\ntab_bar_position="bottom"\ntab_bar_right=[{type="hostname"}]\n')
    script = home / 'remote.sh'
    token = env.get('GH_TOKEN') or env['GITHUB_TOKEN']
    assert re.fullmatch(r'[A-Za-z0-9._~+/=-]+', token), 'invalid token format'
    fetched = subprocess.run(['curl', '-q', '-K', '-', '--silent', '--show-error', '--fail', '--proto', '=https', '--max-time', '120', '-H', 'Accept: application/vnd.github.raw+json', '-o', str(script), f'https://api.github.com/repos/liyixin123/herdr-quota/contents/scripts/remote.sh?ref={bootstrap_ref}'], input=f'header = "Authorization: Bearer {token}"\n', text=True, env=env)
    assert fetched.returncode == 0, 'published bootstrap download failed'
    def run(action):
        subprocess.run(['/bin/sh', str(script), action], check=True, env=env, timeout=300)
    run('install')
    binary = home / '.local/bin/herdr-quota'
    assert subprocess.check_output([str(binary), '--version'], text=True).strip() == 'herdr-quota ' + tag[1:]
    first = config.read_text()
    run('install')
    assert config.read_text() == first
    subprocess.run([str(binary), 'refresh', '--codex', str(tools / 'codex')], env=env, check=True, capture_output=True, timeout=25)
    ui = tomllib.loads(first)['ui']
    command = next(entry['command'] for entry in ui['tab_bar_right'] if entry.get('type') == 'command')
    status = subprocess.run(['/bin/sh', '-c', command], env=env, capture_output=True, text=True, timeout=25)
    assert status.returncode == 0, status.stderr
    assert '72%' in status.stdout and '33%' in status.stdout, status.stdout
    assert ui['tab_bar_position'] == 'bottom'
    assert sum('# herdr-quota' in x.get('command', '') for x in ui['tab_bar_right']) == 1
    binary.unlink()
    (tools / 'codex').unlink()
    run('uninstall')
    run('uninstall')
    assert not binary.exists()
    assert '# herdr-quota' not in config.read_text()
print('Published bootstrap, authenticated native install/repeat/missing-binary uninstall passed.')
