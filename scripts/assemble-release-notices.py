import pathlib,json,hashlib,urllib.request,zipfile,io,shutil,argparse
root=pathlib.Path(__file__).resolve().parents[1]
parser=argparse.ArgumentParser(description='Assemble original publisher notices for Andromeda distributions')
parser.add_argument('--macos-notices',type=pathlib.Path,required=True)
parser.add_argument('--out',type=pathlib.Path,default=root/'release/generated-notices')
args=parser.parse_args();out=args.out.resolve()
assert not out.exists()
shutil.copytree(args.macos_notices.resolve(),out)
manifest=json.loads((out/'inventory.json').read_text());extra=[]
def save(name,data,url):
 p=out/'android'/name;p.parent.mkdir(parents=True,exist_ok=True);p.write_bytes(data);extra.append({'file':str(p.relative_to(out)),'sha256':hashlib.sha256(data).hexdigest(),'source':url})
def fetch(url):
 with urllib.request.urlopen(url,timeout=30) as r:return r.read()
jarurl='https://repo.maven.apache.org/maven2/org/lynxsdk/lynx/lynx/4.1.1/lynx-4.1.1-sources.jar'
jar=fetch(jarurl);assert hashlib.sha256(jar).hexdigest()=='1ac0da6c820a06428cbf76337719275d5368d6898525d2880d2173a6343fe818'
with zipfile.ZipFile(io.BytesIO(jar)) as z:
 for source,name,sha in [('com/lynx/tasm/behavior/LICENSE','lynx-LICENSE.txt','43070e2d4e532684de521b885f385d0841030efa2b1a20bafb76133a5e1379c1'),('com/lynx/tasm/behavior/NOTICE','lynx-NOTICE.txt','5cfae5780d39ffd8b6de6bddacb6385ae03f628f7ac47b49d6a835e1c551733a'),('com/lynx/tasm/utils/LICENSE','lynx-utils-MIT.txt','da6d3703ed11cbe42bd212c725957c98da23cbff1998c05fa4b3d976d1a58e93')]:
  data=z.read(source);assert hashlib.sha256(data).hexdigest()==sha;save(name,data,jarurl+'#'+source)
base='https://raw.githubusercontent.com/lynx-family/primjs/8d129eab500c86e3b2171144e45d3f19e5dce421/'
for path in ['LICENSE','NOTICE','LICENSE.nodejs','LICENSE.v8','third_party/wasm3/LICENSE']:
 save('primjs-'+path.replace('/','-')+'.txt',fetch(base+path),base+path)
url=base+'src/interpreter/quickjs/source/quickjs.cc';data=fetch(url).decode();end=data.index('*/')+2;save('quickjs-header.txt',data[:end].encode()+b'\n',url)
# Preserve the full original notices of runtime JavaScript packages from both UI lockfiles.
for app in ['desktop','android-companion/ui']:
 lock=json.loads((root/app/'package-lock.json').read_text())
 for path,entry in lock['packages'].items():
  if not path or entry.get('dev'):continue
  directory=root/app/path
  if not directory.exists():continue
  package=json.loads((directory/'package.json').read_text())
  for file in directory.iterdir():
   if file.is_file() and any(file.name.upper().startswith(prefix) for prefix in ['LICENSE','LICENCE','NOTICE','COPYING']):
    name=(package['name']+'-'+package['version']+'-'+file.name).replace('/','-').replace('@','')
    save('npm-'+name+'.txt',file.read_bytes(),entry.get('resolved','')+'#'+file.name)
# Supply the exact corresponding Rust source archives for MPL components in the macOS closure.
source_dir=out/'corresponding-source';source_dir.mkdir()
for component in manifest['components']:
 if component['ecosystem']=='cargo' and 'MPL' in (component.get('declared_license') or ''):
  filename=component['name']+'-'+component['version']+'.crate';data=fetch(component['source_archive']);assert hashlib.sha256(data).hexdigest()==component['registry_checksum'];(source_dir/filename).write_bytes(data)
version=json.loads((root/'package.json').read_text())['version']
(out/'SOURCE-ACCESS.txt').write_text('Andromeda '+version+' preview\n\nOriginal dependency notices and source links are recorded in inventory.json and additional-notices.json.\nMPL Rust corresponding-source archives are included under corresponding-source/, with versions and checksums in inventory.json. The crates are unmodified.\nAndromeda source: https://github.com/JeroTan/andromeda\nLynx/PrimJS source associations and provenance qualifications: see the bundled android-native-notices.md.\nThe upstream objc2 LICENSE.md incorporates license texts by reference; preserve that original statement alongside the collected full license texts. No missing copyright holder is invented.\n')
shutil.copy(root/'docs/research/android-native-notices.md',out/'android-native-notices.md')
(out/'additional-notices.json').write_text(json.dumps(extra,indent=2)+'\n')
print(json.dumps({'additional_notice_files':len(extra),'mpl_source_archives':len(list(source_dir.iterdir())),'output':str(out)}))
