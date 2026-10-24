#!/usr/bin/env python3
"""Exercise a paired debug companion's capture protocol; never retain image pixels."""
import argparse
import base64
import hashlib
import json
import pathlib
import socket
import struct
import time


def exact(sock, length):
    result = bytearray()
    while len(result) < length:
        chunk = sock.recv(length-len(result))
        if not chunk:
            raise EOFError('Companion ended its response')
        result.extend(chunk)
    return bytes(result)


class Companion:
    def __init__(self, serial, state):
        self.serial = serial
        name = 'device-'+hashlib.sha256(serial.encode()).hexdigest()+'.json'
        self.token = json.loads((state/name).read_text())['token']

    def connect(self, method, fields):
        sock = socket.create_connection(('127.0.0.1',5037), timeout=6)
        try:
            for service in ['host:transport:'+self.serial, 'localabstract:andromeda-v1']:
                data = service.encode()
                sock.sendall(f'{len(data):04x}'.encode()+data)
                if exact(sock,4) != b'OKAY':
                    raise RuntimeError('ADB companion service unavailable')
            data = json.dumps(dict(version=1,method=method,token=self.token,**fields)).encode()
            sock.sendall(struct.pack('>I',len(data))+data)
            return sock
        except BaseException:
            sock.close()
            raise

    def call(self, method='screenshot', **fields):
        with self.connect(method, fields) as sock:
            length = struct.unpack('>I',exact(sock,4))[0]
            assert 0<length<=4*1024*1024
            return json.loads(exact(sock,length))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--device', required=True)
    parser.add_argument('--state-dir', type=pathlib.Path, required=True)
    parser.add_argument('--output', type=pathlib.Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        parser.error('refusing to overwrite prior evidence')
    peer = Companion(args.device, args.state_dir)
    report = {'schema_version':1, 'pixels_retained':False, 'debug_fault_injection':True}
    first = peer.call(format='jpeg',max_width=720)
    assert first['success'],first.get('error')
    frame = first['data']
    image = base64.b64decode(frame['image_base64'],validate=True)
    assert len(image)==frame['bytes'] and image.startswith(b'\xff\xd8')
    assert frame['width']<=720 and frame['width']<=frame['native_width']
    report['normal_capture'] = {key:frame[key] for key in ['width','height','native_width','native_height','bytes','capture_ms','processing_ms']}
    assert peer.call(format='png',max_width=1.5)['error']['code']=='invalid_argument'
    report['fractional_dimensions_rejected'] = True
    time.sleep(.4)
    delayed = peer.call(format='jpeg',max_width=720,test_callback_delay_ms=3000)
    assert delayed.get('error',{}).get('code')=='timeout', 'Run this probe against a debug APK with callback fault injection'
    busy = peer.call(format='jpeg',max_width=720)
    assert busy.get('error',{}).get('code')=='capture_busy', 'Native slot released before the delayed callback finished'
    report['slot_held_after_timeout'] = True
    deadline = time.monotonic()+6
    while True:
        time.sleep(.4)
        recovered = peer.call(format='jpeg',max_width=720)
        if recovered.get('success'):
            break
        assert recovered.get('error',{}).get('code')=='capture_busy' and time.monotonic()<deadline,recovered.get('error')
    report['late_callback_released_slot'] = True
    time.sleep(.4)
    with peer.connect('screenshot',dict(format='jpeg',max_width=720,test_callback_delay_ms=1000)):
        time.sleep(.1)
    # EOF must discard the frame while retaining native concurrency accounting.
    busy = peer.call(format='jpeg',max_width=720)
    assert busy.get('error',{}).get('code')=='capture_busy',busy.get('error')
    time.sleep(1.4)
    assert peer.call(format='jpeg',max_width=720)['success']
    report['disconnect_during_callback_recovered'] = True
    args.output.parent.mkdir(parents=True,exist_ok=True)
    with args.output.open('x') as output:
        json.dump(report,output,indent=2)
        output.write('\n')
    print(json.dumps({key:value for key,value in report.items() if key!='normal_capture'}))


if __name__ == '__main__':
    main()
