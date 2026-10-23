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
    end=time.monotonic()+20
    while time.monotonic()<end:
        if ready()==expected:return
        time.sleep(.1)
    raise AssertionError(f'Endpoint ready state did not become {expected}')
def app_focused():
    try:
        out=subprocess.check_output([adb,'-s',args.device,'shell','dumpsys','window'],text=True,stderr=subprocess.DEVNULL)
        return 'mCurrentFocus' in out and 'io.andromeda.companion/io.andromeda.companion.MainActivity' in out
    except subprocess.CalledProcessError:return False
command('am','start','-S','-W','-a','android.intent.action.MAIN','-c','android.intent.category.LAUNCHER','-n','io.andromeda.companion/.MainActivity')
end=time.monotonic()+30
while time.monotonic()<end and not app_focused():time.sleep(.25)
if not app_focused():raise AssertionError('Companion UI did not become active before the lifecycle check')
time.sleep(1)
for _ in range(5):
    command('settings','delete','secure','enabled_accessibility_services');await_state(False)
    command('settings','put','secure','enabled_accessibility_services','io.andromeda.companion/io.andromeda.companion.ControlService')
    command('settings','put','secure','accessibility_enabled','1');await_state(True)
print('Five disable/re-enable cycles passed through the ADB companion protocol.')
# Keep the permission entry while making the in-process service handle unavailable. Newer
# Android images clear the enabled-service list when UiAutomation suppresses services, which
# would make this a permission-off test rather than the transient reconnecting state.
result=subprocess.check_output([adb,'-s',args.device,'shell','am','instrument','--user','0','-w','-e','scenario','permission-state','io.andromeda.companion.test/io.andromeda.companion.CompanionUiProbe'],text=True)
assert 'INSTRUMENTATION_RESULT: permission_state=true' in result and 'INSTRUMENTATION_RESULT: status=passed' in result, 'Enabled permission was not recognized while the service was reconnecting: '+result
print('Enabled permission remains recognized while the service handle is unavailable; no repeated setup prompt.')
command('settings','delete','secure','enabled_accessibility_services')
command('settings','put','secure','accessibility_enabled','1')
command('am','force-stop','io.andromeda.companion')
