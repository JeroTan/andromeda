"""Bounded stdio MCP client for local benchmarks; never logs response payloads."""
import json
import os
import selectors
import subprocess
import time


class McpClient:
    def __init__(self, command, *, cwd=None, env=None):
        self.process = subprocess.Popen(command, cwd=cwd, env=env, stdin=subprocess.PIPE,
                                        stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, bufsize=0)
        self.selector = selectors.DefaultSelector()
        self.selector.register(self.process.stdout, selectors.EVENT_READ)
        self.buffer = bytearray()
        self.next_id = 2
        started = time.perf_counter()
        try:
            self.send({'jsonrpc': '2.0', 'id': 1, 'method': 'initialize', 'params': {
                'protocolVersion': '2025-03-26', 'capabilities': {},
                'clientInfo': {'name': 'andromeda-local-benchmark', 'version': '1'}}})
            if 'error' in self.receive(1):
                raise RuntimeError('MCP initialization failed')
            self.send({'jsonrpc': '2.0', 'method': 'notifications/initialized'})
            self.startup_ms = (time.perf_counter() - started) * 1000
        except BaseException:
            self.close()
            raise

    def send(self, value):
        self.process.stdin.write((json.dumps(value) + '\n').encode())
        self.process.stdin.flush()

    def receive(self, request_id, timeout=30):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            if b'\n' in self.buffer:
                line, _, rest = self.buffer.partition(b'\n')
                self.buffer = bytearray(rest)
                value = json.loads(line)
                if value.get('id') == request_id:
                    return value
                continue
            if not self.selector.select(max(0, deadline - time.monotonic())):
                break
            chunk = os.read(self.process.stdout.fileno(), 65536)
            if not chunk:
                raise RuntimeError('MCP server ended before returning a response')
            self.buffer.extend(chunk)
            if len(self.buffer) > 48 * 1024 * 1024:
                raise RuntimeError('MCP response exceeded 48 MiB bound')
        raise TimeoutError('MCP response exceeded the client deadline')

    def tool(self, name, arguments):
        request_id = self.next_id
        self.next_id += 1
        started = time.perf_counter()
        self.send({'jsonrpc': '2.0', 'id': request_id, 'method': 'tools/call',
                   'params': {'name': name, 'arguments': arguments}})
        result = self.receive(request_id)
        return result, (time.perf_counter() - started) * 1000

    def close(self):
        try:
            self.process.stdin.close()
        except (BrokenPipeError, OSError):
            pass
        try:
            self.process.wait(timeout=3)
        except subprocess.TimeoutExpired:
            self.process.terminate()
            try:
                self.process.wait(timeout=3)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait(timeout=3)
        finally:
            self.selector.close()
            self.process.stdout.close()
