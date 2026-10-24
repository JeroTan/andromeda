#!/usr/bin/env python3
"""Matched native-PNG/optional Home acknowledgement comparison; no image retention."""
import argparse
import base64
import hashlib
import json
import math
import os
import pathlib
import platform
import struct
import subprocess
import tempfile
import time
import zlib
from mcp_client import McpClient

ROOT = pathlib.Path(__file__).resolve().parents[1]


def percentile(values, fraction):
    return sorted(values)[max(0, math.ceil(len(values)*fraction)-1)] if values else None


def png_dimensions(data):
    if data[:8] != b'\x89PNG\r\n\x1a\n':
        raise ValueError('PNG signature missing')
    offset = 8
    size = None
    compressed = bytearray()
    finished = False
    channels = None
    while offset + 12 <= len(data):
        length = struct.unpack('>I', data[offset:offset+4])[0]
        kind = data[offset+4:offset+8]
        end = offset+8+length
        if end+4 > len(data):raise ValueError('Truncated PNG chunk')
        body = data[offset+8:end]
        crc = struct.unpack('>I',data[end:end+4])[0]
        if zlib.crc32(kind+body) & 0xffffffff != crc:raise ValueError('PNG chunk checksum mismatch')
        if kind == b'IHDR':
            if offset != 8 or len(body) != 13:raise ValueError('Invalid PNG header')
            width,height,depth,color,compression,filtering,interlace=struct.unpack('>IIBBBBB',body)
            channels={0:1,2:3,3:1,4:2,6:4}.get(color)
            if depth!=8 or not channels or compression or filtering or interlace or not 0<width*height<=8388608:raise ValueError('Unexpected PNG pixel layout')
            size=(width,height)
        elif kind == b'IDAT':compressed.extend(body)
        elif kind == b'IEND':
            if body or end+4 != len(data):raise ValueError('Invalid PNG end')
            finished=True;break
        offset=end+4
    if not finished or not size:raise ValueError('Incomplete PNG')
    expected=(size[0]*channels+1)*size[1]
    inflater=zlib.decompressobj();rows=inflater.decompress(compressed,expected+1)
    if not inflater.eof or inflater.unused_data or inflater.unconsumed_tail or len(rows)!=expected:raise ValueError('Invalid PNG image stream')
    if any(rows[i]>4 for i in range(0,len(rows),size[0]*channels+1)):raise ValueError('Invalid PNG row filter')
    return size


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--andromeda', type=pathlib.Path, required=True)
    parser.add_argument('--state-dir', type=pathlib.Path, required=True)
    parser.add_argument('--packages', type=pathlib.Path, required=True, help='npm ci installation of benchmarks/comparison')
    parser.add_argument('--device', required=True)
    parser.add_argument('--adb', type=pathlib.Path, required=True)
    parser.add_argument('--profile', choices=['debug','release'], required=True, help='Andromeda build profile')
    parser.add_argument('--transport', choices=['usb','emulator','tcp'], required=True)
    parser.add_argument('--samples', type=int, default=20)
    parser.add_argument('--warmup', type=int, default=2)
    parser.add_argument('--reverse', action='store_true')
    parser.add_argument('--allow-input', action='store_true', help='After captures, prepare launcher with Home and measure repeated Home acknowledgements')
    parser.add_argument('--surface', required=True, help='Non-sensitive screen description used for captures')
    parser.add_argument('--output', type=pathlib.Path, required=True)
    args = parser.parse_args()
    if args.output.exists() or not 3 <= args.samples <= 100 or not 0 <= args.warmup <= 10:
        parser.error('Output must be new; samples 3..100; warmup 0..10')
    packages=args.packages.resolve();server=args.andromeda.resolve();state=args.state_dir.resolve();adb=args.adb.resolve()
    lockfile=ROOT/'benchmarks/comparison/package-lock.json'
    expected_lock=json.loads(lockfile.read_text());actual_lock=json.loads((packages/'package-lock.json').read_text())
    if expected_lock != actual_lock:
        raise RuntimeError('Comparison dependency lock does not match the checked-in fixture')
    pins={'@mobilenext/mobile-mcp':'1.0.12','@metamask/device-mcp':'0.5.0'}
    for name,version in pins.items():
        if json.loads((packages/'node_modules'/name/'package.json').read_text())['version'] != version:
            raise RuntimeError('Comparison package version differs from its pin')
    def shell(command):
        return subprocess.check_output([str(adb),'-s',args.device,'shell',command],text=True,timeout=10).strip()
    def battery():
        values={}
        for line in shell('dumpsys battery').splitlines():
            key,sep,value=line.strip().partition(': ')
            if sep and key in ['level','temperature']:
                values[key]=int(value)
        return values
    info=json.loads(subprocess.check_output([str(server),'--device',args.device,'--state-dir',str(state),'call','adm_get_device_info'],text=True,timeout=15))['structuredContent']
    if not info['success']:raise RuntimeError('Cannot read native device geometry')
    screen=info['data']['screen'];dimensions=(screen['width'],screen['height'])
    report={'schema_version':1,'screen':screen,'device':{key:info['data'][key] for key in ['model','manufacturer','android_version','api_level']},'surface':args.surface,'andromeda_profile':args.profile,'transport':args.transport,'host':{'os':platform.system(),'arch':platform.machine(),'node':subprocess.check_output(['node','--version'],text=True).strip()},'server_sha256':hashlib.sha256(server.read_bytes()).hexdigest(),'dependency_lock_sha256':hashlib.sha256(lockfile.read_bytes()).hexdigest(),'package_versions':pins,'samples':args.samples,'warmup':args.warmup,'interval_ms':400,'reverse':args.reverse,'battery_before':battery(),'allow_input':args.allow_input,'workloads':[],'limitations':[
        'Same requested native PNG dimensions and same device; intrinsic geometry checks, encoding, disk files and transport work differ.',
        'Mobile Next is explicitly in documented ADB compatibility mode, not its default JPEG/mobilecli backend. No claim about its default performance.',
        'MetaMask writes temporary host/phone PNGs as part of normal operation; host files are isolated and removed after each workload. Its own phone cleanup remains in timed work.',
        'MCP round trips include JSON/stdout parsing, not model reasoning, remote upload or original frame age. PNG checksum/decompression validation runs outside measured time.',
        'Home acknowledgement is not a gesture-accuracy or presentation-latency measurement. Captures precede any Home input.',
        'Sequential/reverse order exposes ordering variation; no thermal control or product-wide fastest claim.',
        'Telemetry disabled using Mobile Next published environment flag; only inspected local ADB paths are exercised.']}
    implementations=['andromeda_adb','andromeda_companion','mobile_next_adb_compat','metamask_adb']
    if args.reverse:implementations.reverse()
    try:
        for action in (['capture','home'] if args.allow_input else ['capture']):
            if action=='home':
                subprocess.run([str(adb),'-s',args.device,'shell','input','keyevent','3'],check=True,timeout=10,stdout=subprocess.DEVNULL)
                time.sleep(.5)
            for implementation in implementations:
                with tempfile.TemporaryDirectory(prefix='andromeda-mcp-comparison-') as directory:
                    env={key:value for key,value in os.environ.items() if key in ['HOME','LANG','LC_ALL','SYSTEMROOT','WINDIR']}
                    env['PATH']=str(adb.parent)+os.pathsep+os.environ.get('PATH','')
                    env['TMPDIR']=directory;env['TMP']=directory;env['TEMP']=directory
                    env['ANDROID_HOME']=str(adb.parent.parent)
                    if implementation.startswith('andromeda_'):
                        backend=implementation.removeprefix('andromeda_')
                        command=[str(server),'--device',args.device,'--state-dir',str(state),'--capture-backend',backend]
                        if args.allow_input:command.append('--allow-control')
                        command.append('serve')
                        name='adm_get_view' if action=='capture' else 'adm_press_key'
                        arguments={'image':{'format':'png'}} if action=='capture' else {'key':'home'}
                    elif implementation=='mobile_next_adb_compat':
                        env.update(MOBILEMCP_DISABLE_TELEMETRY='1',MOBILEMCP_LEGACY_ROBOT='1')
                        command=['node',str(packages/'node_modules/@mobilenext/mobile-mcp/lib/index.js'),'--stdio']
                        name='mobile_take_screenshot' if action=='capture' else 'mobile_press_button'
                        arguments={'device':args.device,'scale':1} if action=='capture' else {'device':args.device,'button':'HOME'}
                    else:
                        env.update(DEVICE_PLATFORM='android',DEVICE_ID=args.device,DEVICE_MCP_ADB_SNAPSHOT='dump')
                        command=['node',str(packages/'node_modules/@metamask/device-mcp/dist/cli/device-mcp.mjs')]
                        name='device_screenshot' if action=='capture' else 'device_press_button'
                        arguments={} if action=='capture' else {'button':'home'}
                    workload={'implementation':implementation,'action':action,'warmup_samples':[],'samples':[],'complete':False}
                    report['workloads'].append(workload)
                    peer=McpClient(command,cwd=directory,env=env)
                    workload['startup_ms']=peer.startup_ms
                    try:
                        for i in range(args.warmup+args.samples):
                            time.sleep(.4)
                            reply,elapsed=peer.tool(name,arguments)
                            result=reply.get('result',{})
                            sample={'mcp_round_trip_ms':elapsed,'success':'error' not in reply and result.get('isError') is not True}
                            structured=result.get('structuredContent') or {}
                            if implementation.startswith('andromeda_'):
                                sample['success']=sample['success'] and structured.get('success') is True
                                if action=='capture' and sample['success']:
                                    actual=structured['data']['capture_backend'];sample['capture_backend']=actual
                                    if actual!=backend:raise RuntimeError('Andromeda silently selected a different capture backend')
                            if action=='capture' and sample['success']:
                                image=next((c for c in result.get('content',[]) if c.get('type')=='image'),None)
                                if not image:raise RuntimeError('Capture returned no MCP image')
                                pixels=base64.b64decode(image['data'],validate=True)
                                if image['mimeType']!='image/png' or pixels[:8]!=b'\x89PNG\r\n\x1a\n' or len(pixels)<24:raise RuntimeError('Capture did not return the matched PNG encoding')
                                size=png_dimensions(pixels);sample.update(image_bytes=len(pixels),width=size[0],height=size[1])
                                if size!=dimensions:raise RuntimeError('Capture dimensions differ from matched native screen')
                            if not sample['success']:
                                sample['error_code']=structured.get('error',{}).get('code','mcp_tool_error')
                            try:
                                cpu,rss=subprocess.check_output(['ps','-o','%cpu=,rss=','-p',str(peer.process.pid)],text=True,timeout=2).split();sample.update(server_cpu_percent=float(cpu),server_rss_kb=int(rss))
                            except (OSError,ValueError,subprocess.SubprocessError):pass
                            workload['warmup_samples' if i<args.warmup else 'samples'].append(sample)
                            if action=='home' and not sample['success']:raise RuntimeError('Input failed; stopped without retrying')
                        successful=[s['mcp_round_trip_ms'] for s in workload['samples'] if s['success']]
                        workload.update(complete=True,p50_ms=percentile(successful,.5),p95_ms=percentile(successful,.95),failures=sum(not s['success'] for s in workload['samples']))
                        print(json.dumps({k:workload[k] for k in ['implementation','action','p50_ms','p95_ms','failures']}),flush=True)
                    finally:peer.close()
        report['completed_run']=True
    except (RuntimeError,TimeoutError,OSError,ValueError) as error:
        report['completed_run']=False;report['failure']=str(error)
        raise
    finally:
        try:
            report['battery_after']=battery()
        except (OSError, ValueError, subprocess.SubprocessError) as error:
            report['battery_after']=None
            report['battery_after_error']=type(error).__name__
        args.output.parent.mkdir(parents=True,exist_ok=True)
        with args.output.open('x') as file:json.dump(report,file,indent=2);file.write('\n')


if __name__=='__main__':main()
