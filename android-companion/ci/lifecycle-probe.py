import subprocess,socket,struct,json,time,argparse
parser=argparse.ArgumentParser(description='Exercise companion service lifetime on a disposable emulator')
parser.add_argument('--device',required=True)
args=parser.parse_args()
if not args.device.startswith('emulator-'):parser.error('This permission-changing check is restricted to a disposable emulator')
adb='adb'
def command(*shell_args):subprocess.run([adb,'-s',args.device,'shell',*shell_args],check=True,stdout=subprocess.DEVNULL)
def read(s,n):
    b=b''
    while len(b)<n:
        v=s.recv(n-len(b))
        if not v:raise EOFError()
        b+=v
    return b
def ready():
    try:
        with socket.create_connection(('127.0.0.1',5037),timeout=1) as s:
            for value in ['host:transport:'+args.device,'localabstract:andromeda-v1']:
                data=value.encode();s.sendall(f'{len(data):04x}'.encode()+data)
                if read(s,4)!=b'OKAY':return False
            data=json.dumps({'version':1,'method':'status','token':'0'*64}).encode();s.sendall(struct.pack('>I',len(data))+data)
            length=struct.unpack('>I',read(s,4))[0]
            return json.loads(read(s,length))['error']['code']=='unauthorized'
    except (OSError,EOFError,KeyError):return False
def await_state(expected):
    end=time.monotonic()+5
    while time.monotonic()<end:
        if ready()==expected:return
        time.sleep(.05)
    raise AssertionError(f'Endpoint ready state did not become {expected}')
for _ in range(5):
    command('settings','delete','secure','enabled_accessibility_services');await_state(False)
    command('settings','put','secure','enabled_accessibility_services','io.andromeda.companion/io.andromeda.companion.ControlService')
    command('settings','put','secure','accessibility_enabled','1');await_state(True)
print('Five disable/re-enable cycles passed through the ADB companion protocol.')
command('settings','delete','secure','enabled_accessibility_services')
