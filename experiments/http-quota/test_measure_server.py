import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest


class ServerMeasurementTests(unittest.TestCase):
    def test_partial_initialize_line_has_bounded_lifetime(self):
        with tempfile.TemporaryDirectory() as directory:
            fake = Path(directory)/'codex'
            fake.write_text('#!'+sys.executable+'\nimport sys,time\nsys.stdin.readline()\nsys.stdout.write(\'{"id":1\');sys.stdout.flush()\ntime.sleep(30)\n')
            fake.chmod(0o755)
            started = time.monotonic()
            result = subprocess.run([sys.executable, str(Path(__file__).with_name('measure_server.py')), '--initialize-timeout', '0.1'], env=dict(os.environ, PATH=directory+os.pathsep+os.environ.get('PATH','')), capture_output=True, text=True, timeout=2)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('initialize timeout', result.stderr)
            self.assertLess(time.monotonic()-started, 2)

if __name__ == '__main__': unittest.main()
