#!/usr/bin/env python3
"""Linux-only wall/CPU/peak-RSS measurement; output contains only probe status/quotas."""
import argparse
import json
from pathlib import Path
import resource
import subprocess
import sys
import time

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--worker', action='store_true', help=argparse.SUPPRESS)
args = parser.parse_args()
if args.worker:
    start = time.monotonic()
    result = subprocess.run([sys.executable, str(Path(__file__).with_name('probe.py')), '--file-auth'], capture_output=True, text=True)
    usage = resource.getrusage(resource.RUSAGE_CHILDREN)
    print(json.dumps({'wall_seconds': round(time.monotonic()-start, 4), 'cpu_seconds': round(usage.ru_utime+usage.ru_stime, 4), 'peak_rss_mib': round(usage.ru_maxrss/1024, 2), 'result': json.loads(result.stdout)}))
else:
    for _ in range(3):
        subprocess.run([sys.executable, __file__, '--worker'], check=True)
