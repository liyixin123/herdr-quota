import http.server
import json
import pathlib
import subprocess
import sys
import tempfile
import threading
import unittest

SCRIPT = pathlib.Path(__file__).with_name('probe.py')

class ProbeTests(unittest.TestCase):
    def test_direct_query_outputs_only_supported_quota_windows(self):
        class Handler(http.server.BaseHTTPRequestHandler):
            def do_GET(self):
                self.server.seen = (self.path, self.headers.get('Authorization'), self.headers.get('ChatGPT-Account-Id'))
                self.send_response(200); self.end_headers()
                self.wfile.write(json.dumps({'rate_limit': {'primary_window': {'used_percent': 28, 'limit_window_seconds': 18000, 'reset_at': 2000000000}, 'secondary_window': {'used_percent': 10, 'limit_window_seconds': 604800, 'reset_at': 2000000100}}, 'secret': 'response-must-not-leak'}).encode())
            def log_message(self, *args): pass
        with tempfile.TemporaryDirectory() as d, http.server.ThreadingHTTPServer(('127.0.0.1', 0), Handler) as server:
            auth = pathlib.Path(d)/'auth.json'
            auth.write_text(json.dumps({'tokens': {'access_token': 'fixture-access-token', 'account_id': 'fixture-account'}}))
            thread = threading.Thread(target=server.serve_forever, daemon=True); thread.start()
            result = subprocess.run([sys.executable, str(SCRIPT), '--auth-file', str(auth), '--fixture-http', f'http://127.0.0.1:{server.server_port}'], capture_output=True, text=True)
            server.shutdown()
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(json.loads(result.stdout), {'state': 'ready', 'windows': {'5h': {'remaining_percent': 72, 'resets_at': 2000000000}, '1w': {'remaining_percent': 90, 'resets_at': 2000000100}}})
            self.assertEqual(server.seen, ('/wham/usage', 'Bearer fixture-access-token', 'fixture-account'))
            self.assertNotIn('fixture-access-token', result.stdout+result.stderr)
            self.assertNotIn('response-must-not-leak', result.stdout+result.stderr)

    def test_failures_are_sanitized_and_auth_changes_discard_results(self):
        class Handler(http.server.BaseHTTPRequestHandler):
            def do_GET(self):
                self.server.requests += 1
                if self.server.mode == 'timeout':
                    import time
                    time.sleep(0.3)
                    return
                if self.server.mode == 'switch':
                    self.server.auth.write_text(json.dumps({'tokens': {'access_token': 'new-account-secret', 'account_id': 'new-account'}}))
                self.send_response(401 if self.server.mode == 'expired' else 302 if self.server.mode == 'redirect' else 200)
                if self.server.mode == 'redirect': self.send_header('Location', f'http://127.0.0.1:{self.server.server_port}/stolen')
                self.end_headers()
                body = {'rate_limit': {'primary_window': {'used_percent': 25, 'limit_window_seconds': 18000, 'reset_at': 2000000000}}}
                try: self.wfile.write(json.dumps(body).encode())
                except BrokenPipeError: pass
            def log_message(self, *args): pass
        with tempfile.TemporaryDirectory() as d, http.server.ThreadingHTTPServer(('127.0.0.1', 0), Handler) as server:
            auth = pathlib.Path(d)/'auth.json'; server.auth = auth; server.requests = 0
            thread = threading.Thread(target=server.serve_forever, daemon=True); thread.start()
            cases = [('missing', None, 'logged_out_or_unsupported_store'),
                     ('unsafe', {'tokens': {'access_token': 'real-secret', 'account_id': 'real-account'}}, 'unsafe_fixture'),
                     ('redirect', {'tokens': {'access_token': 'fixture-access-token', 'account_id': 'fixture-account'}}, 'http_302'),
                     ('expired', {'tokens': {'access_token': 'fixture-access-token', 'account_id': 'fixture-account'}}, 'auth_expired_or_rejected'),
                     ('switch', {'tokens': {'access_token': 'fixture-access-token', 'account_id': 'fixture-account'}}, 'account_changed_during_query'),
                     ('timeout', {'tokens': {'access_token': 'fixture-access-token', 'account_id': 'fixture-account'}}, 'network_timeout')]
            for mode, data, expected in cases:
                with self.subTest(mode=mode):
                    server.mode = mode
                    if data is not None: auth.write_text(json.dumps(data))
                    before = auth.read_bytes() if auth.exists() else None
                    requests_before = server.requests
                    result = subprocess.run([sys.executable, str(SCRIPT), '--auth-file', str(auth), '--fixture-http', f'http://127.0.0.1:{server.server_port}', '--timeout', '0.1'], capture_output=True, text=True)
                    self.assertEqual(result.returncode, 1)
                    self.assertEqual(json.loads(result.stdout), {'state': expected})
                    self.assertEqual(result.stderr, '')
                    self.assertNotIn('secret', result.stdout)
                    if mode in ('missing', 'unsafe'): self.assertEqual(server.requests, requests_before)
                    if mode == 'redirect': self.assertEqual(server.requests, requests_before + 1)
                    if mode != 'switch': self.assertEqual(auth.read_bytes() if auth.exists() else None, before)
            server.shutdown()

if __name__ == '__main__': unittest.main()
