#!/usr/bin/env python3
"""Versioned release packaging and download verification (Python standard library)."""
import argparse
import hashlib
from pathlib import Path
import re
import subprocess
import tarfile
import urllib.request

TARGETS = ('aarch64-apple-darwin', 'x86_64-unknown-linux-musl')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='command', required=True)
    for name in ('package', 'download'):
        command = commands.add_parser(name)
        command.add_argument('--tag', required=True)
        command.add_argument('--target', choices=TARGETS, required=True)
        command.add_argument('--output', type=Path, required=True)
        if name == 'package':
            command.add_argument('--binary', type=Path, required=True)
        else:
            command.add_argument('--base-url', required=True, help='release download URL, without trailing asset filename')
    args = parser.parse_args()
    if not re.fullmatch(r'v\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?', args.tag):
        parser.error('tag must be vMAJOR.MINOR.PATCH with an optional prerelease suffix')
    args.output.mkdir(parents=True, exist_ok=True)
    name = f'herdr-quota-{args.tag}-{args.target}.tar.gz'
    archive = args.output / name
    if args.command == 'package':
        expected = f'herdr-quota {args.tag[1:]}'
        actual = subprocess.check_output([str(args.binary.resolve()), '--version'], text=True).strip()
        if actual != expected:
            parser.error(f'package version mismatch: expected {expected!r}, got {actual!r}')
        with tarfile.open(archive, 'w:gz') as package:
            info = package.gettarinfo(str(args.binary), arcname='herdr-quota')
            info.mode, info.uid, info.gid, info.uname, info.gname = 0o755, 0, 0, '', ''
            with args.binary.open('rb') as binary:
                package.addfile(info, binary)
        digest = hashlib.sha256(archive.read_bytes()).hexdigest()
        (args.output / 'SHA256SUMS').write_text(f'{digest}  {name}\n')
    else:
        for asset in ('SHA256SUMS', name):
            request = urllib.request.Request(args.base_url.rstrip('/') + '/' + asset)
            with urllib.request.urlopen(request, timeout=60) as response:
                (args.output / asset).write_bytes(response.read())
        entries = {}
        for line in (args.output / 'SHA256SUMS').read_text().splitlines():
            digest, filename = line.split('  ', 1)
            if not re.fullmatch('[0-9a-f]{64}', digest) or filename in entries:
                parser.error('invalid or duplicate checksum entry')
            entries[filename] = digest
        if entries.get(name) != hashlib.sha256(archive.read_bytes()).hexdigest():
            archive.unlink()
            parser.error(f'checksum mismatch or missing entry for {name}')
    print(archive)


if __name__ == '__main__':
    main()
