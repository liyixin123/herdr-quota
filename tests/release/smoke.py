#!/usr/bin/env python3
"""Run an extracted release with an isolated Codex protocol fixture, no Rust."""
import os
from pathlib import Path
import subprocess
import sys
import tempfile

binary = Path(sys.argv[1]).resolve()
version = sys.argv[2].removeprefix('v')
with tempfile.TemporaryDirectory(prefix='herdr-release-') as work:
    directory = Path(work)
    codex = directory / 'codex'
    codex.write_text('''#!/bin/sh
[ "$1" = app-server ] && [ "$2" = --stdio ] || exit 2
while IFS= read -r line; do
  case "$line" in
    *'"method":"initialize"'*) printf '%s\\n' '{"id":1,"result":{}}' ;;
    *'"method":"account/read"'*) printf '%s\\n' '{"id":2,"result":{"userId":"release-fixture-private","account":{"type":"chatgpt","email":"release-fixture-private@example.invalid","planType":"plus"}}}' ;;
    *'"method":"account/rateLimits/read"'*) printf '%s\\n' '{"id":3,"result":{"rateLimits":{"primary":{"windowDurationMins":300,"usedPercent":28,"resetsAt":4102444800},"secondary":{"windowDurationMins":10080,"usedPercent":67,"resetsAt":4102531200}}}}' ;;
  esac
done
''')
    codex.chmod(0o700)
    env = {**os.environ, 'HOME': work, 'PATH': str(directory)}
    def run(*args):
        result = subprocess.run([str(binary), *args], env=env, capture_output=True, text=True, timeout=25)
        assert result.returncode == 0, result.stderr
        assert 'release-fixture-private' not in result.stdout + result.stderr
        return result.stdout.strip()
    assert run('--version') == f'herdr-quota {version}'
    for command in ('refresh', 'status'):
        text = run(command, '--cache-dir', str(directory / 'cache'), '--codex', str(codex))
        assert '72%' in text and '33%' in text and '未知' not in text, text
print('Extracted release version, quota refresh and status passed without Rust in PATH.')
