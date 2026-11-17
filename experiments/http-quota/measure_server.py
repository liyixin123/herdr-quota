#!/usr/bin/env python3
"""Linux-only unauthenticated app-server startup/idle baseline; no quota query."""
import argparse
import json
import math
import signal
import os
from pathlib import Path
import select
import subprocess
import tempfile
import time

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--initialize-timeout', type=float, default=30)
args = parser.parse_args()
if not math.isfinite(args.initialize_timeout) or not 0 < args.initialize_timeout <= 30:
    parser.error('initialize timeout must be between 0 and 30 seconds')

def initialize_timeout(*unused):
    raise RuntimeError('initialize timeout')

signal.signal(signal.SIGALRM, initialize_timeout)

with tempfile.TemporaryDirectory(prefix='quota-server-baseline-') as home:
    start = time.monotonic()
    p = subprocess.Popen(['codex', 'app-server', '--stdio'], env=dict(os.environ, CODEX_HOME=home), stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
    def stats():
        rows = {}
        for path in Path('/proc').glob('[0-9]*/stat'):
            try:
                # Process names can contain spaces: fields after the closing parenthesis.
                parts = path.read_text().rsplit(')', 1)[1].split()
                rows[int(path.parent.name)] = (int(parts[1]), int(parts[11])+int(parts[12]), int(parts[21]))
            except (OSError, ValueError, IndexError):
                pass
        ids = {p.pid}
        while True:
            expanded = ids | {pid for pid, row in rows.items() if row[0] in ids}
            if expanded == ids: break
            ids = expanded
        return {'processes': len(ids), 'rss_mib': round(sum(rows.get(pid, (0,0,0))[2] for pid in ids)*os.sysconf('SC_PAGE_SIZE')/1048576, 2), 'cpu_seconds': sum(rows.get(pid, (0,0,0))[1] for pid in ids)/os.sysconf('SC_CLK_TCK')}
    try:
        signal.setitimer(signal.ITIMER_REAL, max(0.001, args.initialize_timeout-(time.monotonic()-start)))
        p.stdin.write(json.dumps({'id': 1, 'method': 'initialize', 'params': {'clientInfo': {'name': 'quota-baseline', 'version': '1'}}})+'\n'); p.stdin.flush()
        if not select.select([p.stdout], [], [], args.initialize_timeout)[0]: raise RuntimeError('initialize timeout')
        response = json.loads(p.stdout.readline(1_048_577))
        if response.get('id') != 1 or 'error' in response: raise RuntimeError('unexpected initialize response')
        signal.setitimer(signal.ITIMER_REAL, 0)
        initialized = time.monotonic()-start
        p.stdin.write('{"method":"initialized"}\n'); p.stdin.flush()
        time.sleep(2); before = stats(); start_idle = time.monotonic(); time.sleep(15); after = stats()
        print(json.dumps({'initialize_seconds': round(initialized, 4), 'settled': before, 'after_15s_idle': after, 'idle_cpu_percent_one_core': round((after['cpu_seconds']-before['cpu_seconds'])/(time.monotonic()-start_idle)*100, 3)}))
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
        p.terminate()
        try: p.wait(timeout=5)
        except subprocess.TimeoutExpired: p.kill(); p.wait()
