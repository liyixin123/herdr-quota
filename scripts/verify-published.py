#!/usr/bin/env python3
"""Verify actual GitHub Release URLs and checksums using authenticated gh API."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--repo', default='liyixin123/herdr-quota')
parser.add_argument('--tag', required=True)
parser.add_argument('--output', type=Path, required=True)
args = parser.parse_args()
release = json.loads(subprocess.check_output(['gh', 'api', f'repos/{args.repo}/releases/tags/{args.tag}']))
assert release['tag_name'] == args.tag
names = {f'herdr-quota-{args.tag}-{target}.tar.gz' for target in ('aarch64-apple-darwin', 'x86_64-unknown-linux-musl')}
assets = {asset['name']: asset for asset in release['assets']}
assert names | {'SHA256SUMS'} <= assets.keys(), 'release missing required assets'
args.output.mkdir(parents=True, exist_ok=True)
for name in sorted(names | {'SHA256SUMS'}):
    asset = assets[name]
    expected = f'https://github.com/{args.repo}/releases/download/{args.tag}/{name}'
    assert asset['browser_download_url'] == expected, 'unexpected release download URL'
    # API asset downloads also work for private repositories; browser URLs alone
    # return 404 without a logged-in session.
    data = subprocess.check_output(['gh', 'api', f'repos/{args.repo}/releases/assets/{asset["id"]}', '-H', 'Accept: application/octet-stream'])
    (args.output / name).write_bytes(data)
entries = dict(line.split('  ', 1)[::-1] for line in (args.output / 'SHA256SUMS').read_text().splitlines())
assert entries.keys() == names, 'checksum manifest must list exactly the two platform archives'
for name in names:
    assert hashlib.sha256((args.output / name).read_bytes()).hexdigest() == entries[name], f'checksum mismatch: {name}'
print('Both published platform URLs and downloaded SHA256 checksums verified.')
