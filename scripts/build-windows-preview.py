#!/usr/bin/env python3
"""Cross-build an unsigned x64 NSIS preview with a prepared matching Rust GNU sysroot."""
import argparse
import os
import pathlib
import shutil
import subprocess

root = pathlib.Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--toolchain', type=pathlib.Path, required=True,
                    help='Official Rust sysroot containing its host compiler/stdlib and x86_64-pc-windows-gnu stdlib')
parser.add_argument('--notices', type=pathlib.Path, required=True)
args = parser.parse_args()
sysroot = args.toolchain.resolve()
notices = args.notices.resolve()
compiler = sysroot / 'bin/rustc'
if not compiler.is_file() or not (sysroot / 'lib/rustlib/x86_64-pc-windows-gnu/lib').is_dir():
    parser.error('Toolchain must contain the matching compiler and Windows standard library')
if not (notices / 'inventory.json').is_file():
    parser.error('Collect Windows notices before packaging')
if os.environ.get('CARGO_TARGET_DIR'):
    parser.error('Unset CARGO_TARGET_DIR; this packaging script uses the workspace target directory')
needed = {name: shutil.which(name) for name in ['node', 'makensis', 'x86_64-w64-mingw32-gcc', 'x86_64-w64-mingw32-ar']}
if any(path is None for path in needed.values()):
    parser.error('Install Node, NSIS and mingw-w64 build tools first')
env = os.environ.copy()
env['PATH'] = str(sysroot / 'bin') + os.pathsep + env.get('PATH', '')
env['RUSTC'] = str(compiler)
env['CARGO_ENCODED_RUSTFLAGS'] = '\x1f'.join(['--sysroot', str(sysroot)])
env['CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER'] = needed['x86_64-w64-mingw32-gcc']
env['CC_x86_64_pc_windows_gnu'] = needed['x86_64-w64-mingw32-gcc']
env['AR_x86_64_pc_windows_gnu'] = needed['x86_64-w64-mingw32-ar']
env['ANDROMEDA_NOTICES_DIR'] = str(notices)
subprocess.run([needed['node'], 'node_modules/@tauri-apps/cli/tauri.js', 'build',
                '--target', 'x86_64-pc-windows-gnu', '--bundles', 'nsis'],
               cwd=root / 'desktop', env=env, check=True)
