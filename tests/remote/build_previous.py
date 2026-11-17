#!/usr/bin/env python3
"""Build the previous-version lifecycle fixture; developer/CI tooling only."""
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

root = Path(__file__).resolve().parents[2]
output = Path(sys.argv[1]).resolve()
with tempfile.TemporaryDirectory(prefix='hq-previous-') as work:
    work = Path(work)
    shutil.copytree(root / 'src', work / 'src')
    manifest = (root / 'Cargo.toml').read_text()
    import re
    manifest = re.sub(r'(?m)^version = "[^"]+"', 'version = "0.0.0"', manifest, count=1)
    (work / 'Cargo.toml').write_text(manifest)
    lock = (root / 'Cargo.lock').read_text()
    lock = re.sub(r'(name = "herdr-quota"\nversion = ")[^"]+', r'\g<1>0.0.0', lock)
    (work / 'Cargo.lock').write_text(lock)
    args = ['cargo', 'build', '--release', '--locked', '--manifest-path', str(work / 'Cargo.toml')]
    target = os.environ.get('RELEASE_TEST_TARGET')
    if target:
        args += ['--target', target]
    subprocess.run(args, check=True, env={**os.environ, 'CARGO_TARGET_DIR': str(work / 'target')})
    binary = work / 'target'
    if target:
        binary /= target
    output.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(binary / 'release/herdr-quota', output)
print(output)
