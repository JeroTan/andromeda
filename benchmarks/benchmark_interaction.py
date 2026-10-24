#!/usr/bin/env python3
"""Explicit-input benchmark restricted to Andromeda's debug gesture pad and test dialog."""
import argparse
import hashlib
import json
import math
import pathlib
import platform
import re
import subprocess
import time
from mcp_client import McpClient


class ToolFailure(RuntimeError):
    def __init__(self, code):
        self.code = code
        super().__init__("Tool failed: " + code)


def percentile(values, fraction):
    return sorted(values)[max(0, math.ceil(len(values)*fraction)-1)] if values else None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--server', type=pathlib.Path, required=True)
    parser.add_argument('--state-dir', type=pathlib.Path, required=True)
    parser.add_argument('--device', required=True)
    parser.add_argument('--adb', default='adb')
    parser.add_argument('--profile', choices=['debug','release'], required=True)
    parser.add_argument('--samples', type=int, default=12)
    parser.add_argument('--allow-input', action='store_true', required=True)
    parser.add_argument('--output', type=pathlib.Path, required=True)
    args = parser.parse_args()
    if not 3 <= args.samples <= 50 or args.output.exists():
        parser.error('samples must be 3..50 and output must be new')
    server = args.server.resolve()
    peer = McpClient([str(server), '--device', args.device, '--state-dir', str(args.state_dir.resolve()), '--allow-control', 'serve'])
    report = {'schema_version': 1, 'device_serial_retained': False, 'images_retained': False,
              'server_sha256': hashlib.sha256(server.read_bytes()).hexdigest(), 'profile': args.profile, 'host_os': platform.system(), 'host_arch': platform.machine(), 'emulator': args.device.startswith('emulator-'), 'samples_per_workload': args.samples, 'warmup_per_workload': 0,
              'workloads': [], 'observation_retries': {}, 'limitations': [
                  'One explicit debug pad; results do not establish accuracy across arbitrary apps or OS versions.',
                  'Native MotionEvent endpoints and duration are measured by the debug view; diagnostics may add overhead.',
                  'MCP round trip includes dispatch and transport; verification polling and model thinking are excluded from it.',
                  'No retries for input. Observation polls may wait for completion. Input points stay inside the observed debug pad.',
                  'Dialog completion and Back-key recovery are deterministic test tasks, not model task success rates.']}

    def call(name, arguments, timeout_ms=10000):
        response, elapsed = peer.tool(name, dict(arguments, timeout_ms=timeout_ms))
        structured = response.get('result', {}).get('structuredContent', {})
        if structured.get('success') is not True:
            raise ToolFailure(structured.get('error', {}).get('code', 'mcp_protocol'))
        return structured['data'], elapsed

    def tree():
        deadline = time.monotonic()+3
        while True:
            try:
                return call('adm_get_ui_tree', {}, timeout_ms=max(1, int((deadline-time.monotonic())*1000)))[0]
            except ToolFailure as error:
                if error.code != 'ui_unavailable' or time.monotonic() >= deadline:
                    raise
                counts = report['observation_retries']
                counts[error.code] = counts.get(error.code, 0)+1
                time.sleep(.05)

    def pad():
        ui = tree()
        node = next((n for n in ui['nodes'] if n['resource_id'].endswith(':id/gesture_pad') and n['package'] == 'io.andromeda.companion'), None)
        if not node:
            raise RuntimeError('Expected the visible Andromeda debug gesture pad; input stopped')
        if 'gesture_json=' not in node['description']:
            raise RuntimeError('Update the debug APK before measuring gesture endpoints')
        metadata, payload = node['description'].split('gesture_json=', 1)
        counts = {key: int(value) for key, value in re.findall(r'(\w+)=(\d+)', metadata)}
        return ui, node, counts, json.loads(payload)

    def tap_label(label, ui):
        node = next((n for n in ui['nodes'] if n['package'] == 'io.andromeda.companion' and (n['text'].casefold() == label.casefold() or n['description'].casefold() == label.casefold())), None)
        if not node or not node['bounds'] or not node['enabled']:
            raise RuntimeError('Missing enabled test control: ' + label)
        left, top, right, bottom = node['bounds']
        return call('adm_touch', {'point': {'x': (left+right)//2, 'y': (top+bottom)//2}, 'screen': ui['screen']})

    try:
        info, _ = call('adm_get_device_info', {})
        report['device'] = {key: info[key] for key in ['model', 'manufacturer', 'android_version', 'api_level', 'screen']}
        package = subprocess.check_output([args.adb, '-s', args.device, 'shell', 'pm path io.andromeda.companion'], text=True).strip().split('package:')[-1]
        report['apk_sha256'] = subprocess.check_output([args.adb, '-s', args.device, 'shell', 'sha256sum', package], text=True).split()[0]
        for kind in ['tap', 'swipe', 'two_finger', 'capture_then_tap']:
            samples = []
            workload = {'name': kind, 'samples': samples, 'complete': False}
            report['workloads'].append(workload)
            for index in range(args.samples):
                ui, node, counts, _ = pad()
                left, top, right, bottom = node['bounds']
                x1, x2 = left+(right-left)//3, left+2*(right-left)//3
                y1, y2 = top+(bottom-top)//3, top+2*(bottom-top)//3
                if min(right-left, bottom-top) < 120:
                    raise RuntimeError('Gesture pad too small')
                duration = [250, 500, 1000][index % 3]
                if kind in ['tap','capture_then_tap']:
                    x = left+(1+index%3)*(right-left)//4
                    y = top+(1+(index//3)%3)*(bottom-top)//4
                    expected = [({'x': x, 'y': y}, {'x': x, 'y': y})]
                    name, arguments = 'adm_touch', {'point': expected[0][0]}
                elif kind == 'swipe':
                    a, b = ({'x': x1, 'y': y1}, {'x': x2, 'y': y2}) if index%2 == 0 else ({'x': x2, 'y': y2}, {'x': x1, 'y': y1})
                    expected = [(a, b)]
                    name, arguments = 'adm_swipe', {'swipe': {'from': a, 'to': b, 'duration_ms': duration}}
                else:
                    expected = [({'x': x1, 'y': y1}, {'x': x1, 'y': y2}), ({'x': x2, 'y': y1}, {'x': x2, 'y': y2})]
                    name, arguments = 'adm_multi_touch', {'strokes': [{'points': [a, b], 'start_ms': 0, 'duration_ms': duration} for a, b in expected]}
                sample = {'requested_tool': name, 'requested_arguments': dict(arguments, screen=ui['screen']), 'completed': False}
                samples.append(sample)
                if kind == 'capture_then_tap':
                    time.sleep(.4)
                started = time.perf_counter()
                if kind == 'capture_then_tap':
                    captured, capture_elapsed = call('adm_get_view', {'image': {'format':'jpeg','max_width':720,'quality':80}})
                    if captured['screen'] != ui['screen']:
                        raise RuntimeError('Screen changed between screenshot and planned tap')
                    sample['capture_mcp_ms'] = capture_elapsed
                    sample['capture_backend'] = captured['capture_backend']
                _, latency = call(name, dict(arguments, screen=ui['screen']))
                if kind == 'capture_then_tap':
                    sample['screenshot_to_action_ack_ms'] = (time.perf_counter()-started)*1000
                deadline = time.monotonic()+3
                polls = 0
                while True:
                    observed, _, actual_counts, traces = pad();polls += 1
                    if observed['screen'] != ui['screen']:
                        raise RuntimeError('Screen changed during gesture measurement')
                    if actual_counts['completed'] == counts['completed']+1:
                        break
                    if actual_counts['completed'] > counts['completed']+1 or time.monotonic() >= deadline:
                        raise RuntimeError('Expected exactly one newly completed gesture')
                    time.sleep(.05)
                errors = []
                # Match pointers by their nearest starting point, independent of Android pointer IDs.
                available = list(traces)
                if len(available) != len(expected):
                    raise RuntimeError('Native pointer count differs from requested stroke count')
                durations = []
                for a, b in expected:
                    trace = min(available, key=lambda p: math.hypot(p['start_x']-a['x'], p['start_y']-a['y']));available.remove(trace)
                    errors.extend([math.hypot(trace['start_x']-a['x'], trace['start_y']-a['y']), math.hypot(trace['end_x']-b['x'], trace['end_y']-b['y'])])
                    durations.append(trace['duration_ms'])
                sample.update({'completed': True, 'mcp_round_trip_ms': latency, 'verified_wall_ms': (time.perf_counter()-started)*1000, 'observation_polls': polls, 'endpoint_errors_px': errors, 'requested_duration_ms': None if kind in ['tap','capture_then_tap'] else duration, 'observed_durations_ms': durations, 'native_traces': traces, 'duration_errors_ms': [] if kind in ['tap','capture_then_tap'] else [abs(d-duration) for d in durations], 'endpoint_within_2px': max(errors) <= 2, 'completed_delta': 1})
            latencies = [s['mcp_round_trip_ms'] for s in samples]
            workload.update({'name': kind, 'samples': samples, 'completed': len(samples), 'endpoint_within_2px': sum(s['endpoint_within_2px'] for s in samples), 'max_endpoint_error_px': max(max(s['endpoint_errors_px']) for s in samples), 'p50_mcp_ms': percentile(latencies,.5), 'p95_mcp_ms': percentile(latencies,.95), 'complete': True})
            print(json.dumps({k:v for k,v in workload.items() if k!='samples'}), flush=True)
        report['tasks'] = []
        for recovery in [False, True]:
            for index in range(3):
                ui, _, before, _ = pad();started = time.perf_counter()
                tap_label('Open test dialog', ui)
                deadline = time.monotonic()+3
                dialog_polls = 0
                while True:
                    dialog = tree();dialog_polls += 1
                    if any(n['text'] == 'Andromeda test dialog' for n in dialog['nodes']):
                        break
                    if time.monotonic() >= deadline:
                        raise RuntimeError('Test dialog did not open within 3 seconds')
                    time.sleep(.05)
                if recovery:
                    call('adm_press_key', {'key':'back'})
                else:
                    tap_label('Close test dialog', dialog)
                deadline = time.monotonic()+3
                return_polls = 0
                while True:
                    observed = tree();return_polls += 1
                    if any(n['resource_id'].endswith(':id/gesture_pad') for n in observed['nodes']):
                        _, _, after, _ = pad()
                        break
                    if time.monotonic() >= deadline:
                        raise RuntimeError('Test pad did not return within 3 seconds')
                    time.sleep(.05)
                expected = before['dialogs_closed']+(0 if recovery else 1)
                if after['dialogs_closed'] != expected:
                    raise RuntimeError('Dialog completion state is incorrect')
                report['tasks'].append({'kind': 'recover_with_back' if recovery else 'open_close_dialog', 'completed': True, 'elapsed_ms': (time.perf_counter()-started)*1000, 'input_retries': 0, 'dialog_observation_polls': dialog_polls, 'return_observation_polls': return_polls})
        report['completed_run'] = True
        report['all_endpoints_within_2px'] = all(w['endpoint_within_2px'] == len(w['samples']) for w in report['workloads'])
        print(json.dumps({'tasks_completed': len(report['tasks']), 'all_endpoints_within_2px': report['all_endpoints_within_2px']}))
    except (RuntimeError, TimeoutError, OSError, ValueError) as error:
        report['completed_run'] = False
        report['failure'] = str(error)
        raise
    finally:
        peer.close()
        args.output.parent.mkdir(parents=True,exist_ok=True)
        with args.output.open('x') as file:json.dump(report,file,indent=2);file.write('\n')



if __name__ == '__main__':
    main()
