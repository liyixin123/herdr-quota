#!/usr/bin/env python3
"""Read-only HTTP feasibility probe. Does not refresh or persist credentials."""
import argparse
import hashlib
import http.client
import json
import math
import os
from pathlib import Path
import socket
import signal
import time
from urllib.parse import urlsplit
import urllib.request
import urllib.error

MAX_BYTES = 1024 * 1024

class Failure(Exception):
    pass

def credentials(path):
    try:
        raw = path.read_bytes()
        if len(raw) > MAX_BYTES:
            raise ValueError()
        data = json.loads(raw)
        if data.get('auth_mode') not in (None, 'chatgpt'):
            raise Failure('unsupported_auth')
        tokens = data.get('tokens') or {}
        access, account = tokens.get('access_token'), tokens.get('account_id')
        if not isinstance(access, str) or not access or not isinstance(account, str) or not account:
            raise Failure('logged_out_or_unsupported_store')
        if any(c in access+account for c in '\r\n'):
            raise ValueError()
        return access, account, hashlib.sha256(raw).digest()
    except FileNotFoundError:
        raise Failure('logged_out_or_unsupported_store') from None
    except (OSError, ValueError, TypeError, AttributeError):
        raise Failure('invalid_auth') from None

def normalize(data):
    try:
        windows = {}
        for window in (data['rate_limit'].get('primary_window'), data['rate_limit'].get('secondary_window')):
            if window is None:
                continue
            name = {18000: '5h', 604800: '1w'}.get(window['limit_window_seconds'])
            if name is None:
                continue
            used, reset = window['used_percent'], window['reset_at']
            if isinstance(used, bool) or not isinstance(used, (int, float)) or not math.isfinite(used) or not 0 <= used <= 100:
                raise ValueError()
            if isinstance(reset, bool) or not isinstance(reset, int) or reset < 0 or name in windows:
                raise ValueError()
            windows[name] = {'remaining_percent': 100-used, 'resets_at': reset}
        if not windows:
            raise ValueError()
        return {'state': 'ready', 'windows': windows}
    except (KeyError, TypeError, ValueError, AttributeError):
        raise Failure('unsupported_response') from None

def run(args):
    if not args.fixture_http and not args.file_auth:
        raise Failure('file_auth_confirmation_required')
    access, account, fingerprint = credentials(args.auth_file)
    # A test endpoint can receive only these public fixture values, never real credentials.
    if args.fixture_http:
        parsed = urlsplit(args.fixture_http)
        if (parsed.scheme != 'http' or parsed.hostname != '127.0.0.1' or parsed.username or parsed.password
                or parsed.path or parsed.query or parsed.fragment
                or access != 'fixture-access-token' or account != 'fixture-account'):
            raise Failure('unsafe_fixture')
        url = args.fixture_http + '/wham/usage'
        handlers = [urllib.request.ProxyHandler({})]
    else:
        url = 'https://chatgpt.com/backend-api/wham/usage'
        handlers = []
    deadline = time.monotonic() + args.timeout
    try:
        class NoRedirect(urllib.request.HTTPRedirectHandler):
            def redirect_request(self, *unused):
                return None
        opener = urllib.request.build_opener(*handlers, NoRedirect())
        request = urllib.request.Request(url, headers={'Authorization': 'Bearer '+access, 'ChatGPT-Account-Id': account, 'Accept': 'application/json', 'User-Agent': 'herdr-quota-http-probe/0.1'})
        response = opener.open(request, timeout=args.timeout)
        body = bytearray()
        while len(body) <= MAX_BYTES:
            remaining = deadline-time.monotonic()
            if remaining <= 0:
                raise Failure('network_timeout')
            part = response.read1(min(65536, MAX_BYTES+1-len(body)))
            if not part:
                break
            body.extend(part)
        if len(body) > MAX_BYTES:
            raise Failure('response_too_large')
        try:
            result = normalize(json.loads(body))
        except (ValueError, UnicodeError):
            raise Failure('unsupported_response') from None
        if credentials(args.auth_file)[2] != fingerprint:
            raise Failure('account_changed_during_query')
        return result
    except urllib.error.HTTPError as error:
        error.close()
        raise Failure('auth_expired_or_rejected' if error.code == 401 else 'http_'+str(error.code)) from None
    except urllib.error.URLError as error:
        raise Failure('network_timeout' if isinstance(error.reason, (socket.timeout, TimeoutError)) else 'network_error') from None
    except (socket.timeout, TimeoutError):
        raise Failure('network_timeout') from None
    except (OSError, http.client.HTTPException):
        raise Failure('network_error') from None
    finally:
        if 'response' in locals():
            response.close()

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--auth-file', type=Path, default=Path(os.environ.get('CODEX_HOME', str(Path.home()/'.codex')))/'auth.json')
    parser.add_argument('--file-auth', action='store_true', help='Confirm this auth.json is the active file-backed Codex credential store (keyring/auto are unsupported)')
    parser.add_argument('--timeout', type=float, default=10)
    parser.add_argument('--fixture-http', help=argparse.SUPPRESS)
    args = parser.parse_args()
    if not math.isfinite(args.timeout) or not 0 < args.timeout <= 30:
        parser.error('timeout must be between 0 and 30 seconds')
    def timeout_handler(*unused):
        raise Failure('network_timeout')
    signal.signal(signal.SIGALRM, timeout_handler)
    signal.setitimer(signal.ITIMER_REAL, args.timeout)
    try:
        result = run(args)
        code = 0
    except Failure as error:
        result = {'state': str(error)}
        code = 1
    except Exception:
        result = {'state': 'probe_error'}
        code = 1
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
    print(json.dumps(result, separators=(',', ':')))
    return code

if __name__ == '__main__':
    raise SystemExit(main())
