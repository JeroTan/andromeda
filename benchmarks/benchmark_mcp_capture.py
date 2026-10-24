#!/usr/bin/env python3
"""Observe-only MCP capture benchmark. Images are parsed in memory and never written."""
import argparse
import hashlib
import json
import math
import os
import pathlib
import platform
import selectors
import shlex
import subprocess
import time


def percentile(values, fraction):
    values = sorted(values)
    return values[max(0, min(len(values)-1, math.ceil(len(values)*fraction)-1))] if values else None


def device_shell(args, command):
    return subprocess.check_output([args.adb, '-s', args.device, 'shell', command], timeout=10, text=True)


def battery(args):
    fields = {}
    for line in device_shell(args, 'dumpsys battery').splitlines():
        key, sep, value = line.strip().partition(': ')
        if sep and key in ('level', 'temperature'):
            try:
                fields['level_percent' if key == 'level' else 'temperature_c'] = int(value) if key == 'level' else int(value)/10
            except ValueError:
                pass
    return fields


def apk_digest(args):
    try:
        entries=device_shell(args,'pm path io.andromeda.companion').splitlines()
        path=next(line.removeprefix('package:') for line in entries if line.endswith('/base.apk'))
        digest=device_shell(args,'sha256sum '+shlex.quote(path)).split()[0]
        return digest if len(digest)==64 and all(ch in '0123456789abcdef' for ch in digest) else None
    except (OSError, StopIteration, IndexError, subprocess.SubprocessError):
        return None


def revision():
    try:
        root=pathlib.Path(__file__).resolve().parents[1]
        sha=subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip()
        dirty=bool(subprocess.check_output(['git','status','--porcelain'],cwd=root,text=True).strip())
        return {'git_head':sha,'working_tree_dirty':dirty}
    except (OSError,subprocess.SubprocessError):
        return None


class Peer:
    def __init__(self, args, backend):
        command = [str(args.server), '--device', args.device, '--state-dir', str(args.state_dir), '--capture-backend', backend, 'serve']
        self.process = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, bufsize=0)
        self.selector = selectors.DefaultSelector()
        self.selector.register(self.process.stdout, selectors.EVENT_READ)
        self.buffer = bytearray()
        started=time.perf_counter()
        try:
            self.send({'jsonrpc':'2.0','id':1,'method':'initialize','params':{'protocolVersion':'2025-03-26','capabilities':{},'clientInfo':{'name':'capture-benchmark','version':'1'}}})
            initialized=self.receive(1)
            if 'error' in initialized:
                raise RuntimeError('MCP initialization failed')
            self.send({'jsonrpc':'2.0','method':'notifications/initialized'})
            self.startup_ms=(time.perf_counter()-started)*1000
            self.next_id=2
        except BaseException:
            self.close()
            raise

    def send(self, value):
        self.process.stdin.write((json.dumps(value)+'\n').encode())
        self.process.stdin.flush()

    def receive(self, request_id):
        deadline = time.monotonic()+20
        while time.monotonic()<deadline:
            if b'\n' in self.buffer:
                line, _, rest = self.buffer.partition(b'\n')
                self.buffer = bytearray(rest)
                value = json.loads(line)
                if value.get('id') == request_id:
                    return value
                continue
            if not self.selector.select(max(0, deadline-time.monotonic())):
                break
            chunk = os.read(self.process.stdout.fileno(), 65536)
            if not chunk:
                raise RuntimeError('MCP server ended before returning a response')
            self.buffer.extend(chunk)
            if len(self.buffer)>48*1024*1024:
                raise RuntimeError('MCP response exceeds benchmark bound')
        raise TimeoutError('MCP capture response exceeded 20 seconds')

    def capture(self, image, backend):
        request_id = self.next_id
        self.next_id += 1
        started = time.perf_counter()
        self.send({'jsonrpc':'2.0','id':request_id,'method':'tools/call','params':{'name':'adm_get_view','arguments':{'image':image,'timeout_ms':15000}}})
        reply = self.receive(request_id)
        elapsed = (time.perf_counter()-started)*1000
        result = reply.get('result', {}).get('structuredContent', {})
        sample = {'mcp_round_trip_ms':elapsed, 'success':result.get('success', False)}
        if not sample['success']:
            sample['error_code'] = result.get('error', {}).get('code', 'mcp_protocol')
            return sample
        data = result['data']
        if data['capture_backend'] != backend:
            raise RuntimeError('Unexpected capture backend; comparisons must never include silent fallback')
        sample.update(screen=data['screen'], server_elapsed_ms=result['elapsed_ms'], mcp_framing_and_client_ms=elapsed-result['elapsed_ms'], image_bytes=data['bytes'], image_width=data['image_width'], image_height=data['image_height'], timings=data['timings'])
        try:
            cpu, rss = subprocess.check_output(['ps','-o','%cpu=,rss=','-p',str(self.process.pid)], text=True, timeout=2).split()
            sample.update(server_cpu_percent=float(cpu), server_rss_kb=int(rss))
        except (OSError, ValueError, subprocess.SubprocessError):
            pass
        return sample

    def close(self):
        self.process.stdin.close()
        try:
            self.process.wait(timeout=3)
        except subprocess.TimeoutExpired:
            self.process.terminate()
            try:
                self.process.wait(timeout=3)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait(timeout=3)
        self.selector.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--server', type=pathlib.Path, required=True)
    parser.add_argument('--state-dir', type=pathlib.Path, required=True)
    parser.add_argument('--device', required=True)
    parser.add_argument('--adb', default='adb')
    parser.add_argument('--profile', choices=['debug','release'], required=True)
    parser.add_argument('--samples', type=int, default=20)
    parser.add_argument('--warmup', type=int, default=2)
    parser.add_argument('--interval-ms', type=int, default=400)
    parser.add_argument('--reverse', action='store_true')
    parser.add_argument('--output', type=pathlib.Path, required=True)
    args = parser.parse_args()
    if not 3<=args.samples<=200 or not 0<=args.warmup<=20 or not 350<=args.interval_ms<=5000:
        parser.error('samples 3..200, warmup 0..20, interval-ms 350..5000')
    if args.output.exists():
        parser.error('output already exists; refusing to overwrite evidence')
    args.server = args.server.resolve()
    args.state_dir = args.state_dir.resolve()
    props = device_shell(args, 'getprop ro.product.model; getprop ro.build.version.release; getprop ro.build.version.sdk').splitlines()
    report = {'schema_version':1, 'metadata':{'model':props[0], 'android_version':props[1], 'api_level':props[2], 'host_os':platform.system(), 'host_arch':platform.machine(), 'profile':args.profile, 'binary_sha256':hashlib.sha256(args.server.read_bytes()).hexdigest(), 'companion_apk_sha256':apk_digest(args), 'host_os_release':platform.release(), 'source':revision(), 'samples_per_workload':args.samples, 'warmup_per_workload':args.warmup, 'interval_ms':args.interval_ms, 'reverse':args.reverse, 'battery_before':battery(args)}, 'workloads':[], 'limitations':['Read-only captures; no input actions and no image retention.', 'MCP round trip includes JSON serialization, stdio transfer and Python parsing; no model thinking or remote client upload.', 'CPU percent is ps lifetime averaging for each server process, not isolated per-frame utilization. RSS is sampled after each response.', 'Nominal quality 80 and matching dimensions; Android and Rust codec/resizer implementations are not perceptually calibrated.', 'Sequential workloads; repeat in reverse order to examine thermal/order effects.', 'API timestamp age uses the device uptime clock and does not prove original presentation age.']}
    backend_order = ['companion','adb'] if args.reverse else ['adb','companion']
    for image in [{'format':'png'}, {'format':'jpeg','max_width':720,'quality':80}, {'format':'webp','max_width':720,'quality':80}]:
        for backend in backend_order:
            peer = Peer(args, backend)
            samples = []
            warmups = []
            try:
                for i in range(args.warmup+args.samples):
                    time.sleep(args.interval_ms/1000)
                    sample = peer.capture(image, backend)
                    if i>=args.warmup:
                        samples.append(sample)
                    else:
                        warmups.append(sample)
            finally:
                peer.close()
            successes = [s['mcp_round_trip_ms'] for s in samples if s['success']]
            workload = {'server_startup_ms':peer.startup_ms, 'warmup_samples':warmups, 'backend':backend, 'image':image, 'p50_ms':percentile(successes,.5), 'p95_ms':percentile(successes,.95), 'failures':sum(not s['success'] for s in samples), 'samples':samples}
            report['workloads'].append(workload)
            print(json.dumps({key:workload[key] for key in ['backend','image','p50_ms','p95_ms','failures']}), flush=True)
    report['metadata']['battery_after'] = battery(args)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open('x') as output:
        json.dump(report, output, indent=2)
        output.write('\n')
    if any(workload['failures'] for workload in report['workloads']):
        raise SystemExit('Capture failures recorded; inspect the report before making performance claims')


if __name__ == '__main__':
    main()
