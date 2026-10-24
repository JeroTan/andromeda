#!/usr/bin/env python3
"""Collect reviewable macOS notice inputs; never label an incomplete inventory complete."""
import argparse
import hashlib
import json
import pathlib
import re
import shutil
import subprocess
import tempfile
import tomllib

ROOT = pathlib.Path(__file__).resolve().parents[1]
NOTICE = re.compile(r'^(?:licen[cs]e|copying|notice|copyright|unlicense|patents|authors)(?:[._-].*)?$', re.I)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def collect(about, output):
    if output.exists():
        raise ValueError('Output exists; refusing to overwrite')
    version = subprocess.check_output([str(about), '--version'], text=True).strip()
    if version != 'cargo-about 0.9.2':
        raise ValueError('Install cargo-about 0.9.2 with --features cli')
    with tempfile.TemporaryDirectory(prefix='andromeda-notices-') as directory:
        work = pathlib.Path(directory)
        raw = work / 'rust.json'
        subprocess.run([str(about), 'generate', '--locked', '--fail', '--format', 'json', '--workspace', '--config', str(ROOT / 'release/about.toml'), '--output-file', str(raw)], cwd=ROOT, check=True)
        generated = json.loads(raw.read_text())
        staged = work / 'notices'
        (staged / 'texts').mkdir(parents=True)
        overrides = json.loads((ROOT / 'release/notice-overrides.json').read_text())
        overrides = {(p['name'], p['version']): p for p in overrides['components']}
        entries = []
        gaps = []
        text_count = set()

        def save(data):
            sha = digest(data)
            name = 'texts/' + sha + '.txt'
            (staged / name).write_bytes(data)
            text_count.add(sha)
            return {'file': name, 'sha256': sha}

        # Retain complete tool-produced license text, not just SPDX identifiers.
        locked = tomllib.loads((ROOT / 'Cargo.lock').read_text())
        checksums = {(p['name'], p['version']): p.get('checksum') for p in locked['package']}
        collected = {}
        for license in generated['licenses']:
            ref = save(license['text'].encode())
            for used in license['used_by']:
                key = (used['crate']['name'], used['crate']['version'])
                original = used.get('path')
                if original and pathlib.Path(original).is_absolute():
                    base = pathlib.Path(used['crate']['manifest_path']).parent
                    original = pathlib.Path(original).relative_to(base).as_posix()
                collected.setdefault(key, []).append(dict(ref, license=license['id'], original_path=original))
        for crate in generated['crates']:
            package = crate['package']
            if package['source'] is None:
                continue
            key = (package['name'], package['version'])
            source = pathlib.Path(package['manifest_path']).parent
            originals = []
            # Include nested native notices (e.g. libwebp), preserving relative paths.
            for path in sorted(source.rglob('*')):
                if path.is_file() and not path.is_symlink() and NOTICE.match(path.name):
                    originals.append(dict(save(path.read_bytes()), original_path=path.relative_to(source).as_posix()))
            checksum = checksums[key]
            archive = source.parents[2] / 'cache' / source.parent.name / f'{key[0]}-{key[1]}.crate'
            if not checksum or digest(archive.read_bytes()) != checksum:
                raise ValueError(f'Cached crate archive does not match Cargo.lock: {key}')
            override = overrides.get(key)
            verified_original = bool(originals)
            if override:
                if override['registry_checksum'] != checksum or digest((source / '.cargo_vcs_info.json').read_bytes()) != override['vcs_info_sha256']:
                    raise ValueError(f'Notice override does not match the exact locked source: {key}')
                for original in override['original_files']:
                    data = (ROOT / original['file']).read_bytes()
                    if digest(data) != original['sha256']:
                        raise ValueError(f'Notice override hash differs: {key}')
                    originals.append(dict(save(data), original_path=original['original_path'], upstream_url=original['upstream_url'], role=original['role']))
                verified_original = verified_original or override['status'] == 'original_texts_verified'
            entries.append({'ecosystem': 'cargo', 'name': key[0], 'version': key[1], 'declared_license': package.get('license'), 'tool_license_expression': crate['license'], 'registry_checksum': checksum, 'source_archive': f'https://static.crates.io/crates/{key[0]}/{key[0]}-{key[1]}.crate', 'notices': collected.get(key, []), 'original_files': originals})
            if not verified_original:
                gaps.append({'component': f'{key[0]}@{key[1]}', 'reason': override.get('gap') if override else 'No original notice file in the published crate. Tool-generated text needs copyright/source review.'})
        lock = json.loads((ROOT / 'desktop/package-lock.json').read_text())
        for path, metadata in sorted(lock['packages'].items()):
            if not path or metadata.get('dev'):
                continue
            source = ROOT / 'desktop' / path
            package = json.loads((source / 'package.json').read_text())
            originals = []
            for file in sorted(source.rglob('*')):
                relative = file.relative_to(source)
                if 'node_modules' in relative.parts:
                    continue
                if file.is_file() and not file.is_symlink() and NOTICE.match(file.name):
                    originals.append(dict(save(file.read_bytes()), original_path=relative.as_posix()))
            entries.append({'ecosystem': 'npm', 'name': package['name'], 'version': package['version'], 'declared_license': package.get('license'), 'integrity': metadata.get('integrity'), 'source_archive': metadata.get('resolved'), 'original_files': originals})
            if not originals:
                gaps.append({'component': f"{package['name']}@{package['version']}", 'reason': 'No packaged license/notice file found'})
        gaps.append({'component': 'desktop build output', 'reason': 'Reconcile generated runtime/helpers and embedded assets with this inventory before public release; runtime npm metadata alone is not proof of bundle contents.'})
        inventory = {'schema_version': 1, 'complete': False, 'scope': 'macOS aarch64 Rust workspace default features, build dependencies retained, dev dependencies omitted; desktop npm runtime packages', 'collector': version, 'cargo_lock_sha256': digest((ROOT / 'Cargo.lock').read_bytes()), 'npm_lock_sha256': digest((ROOT / 'desktop/package-lock.json').read_bytes()), 'config_sha256': digest((ROOT / 'release/about.toml').read_bytes()), 'overrides_sha256': digest((ROOT / 'release/notice-overrides.json').read_bytes()), 'components': entries, 'review_gaps': gaps, 'limitations': ['This is a review candidate, not a legal-compliance conclusion or full-release approval.', 'Android/Lynx native artifacts and Windows/Linux target-specific closures require separate inventories.', 'MPL corresponding-source distribution and source-access instructions remain a public release requirement.']}
        (staged / 'inventory.json').write_text(json.dumps(inventory, indent=2, sort_keys=True) + '\n')
        lines = ['Andromeda third-party notice review candidate', '', 'Completeness: pending; see inventory.json review_gaps.', 'Full collected texts are in texts/, addressed by SHA-256.', 'Original relative paths and source archive checksums are retained in inventory.json.', '']
        for entry in entries:
            lines.append(f"{entry['ecosystem']}: {entry['name']} {entry['version']} — {entry.get('declared_license')}")
            lines.extend('  ' + item['file'] for item in entry.get('notices', []) + entry['original_files'])
        (staged / 'THIRD-PARTY-NOTICES.txt').write_text('\n'.join(lines) + '\n')
        output.parent.mkdir(parents=True, exist_ok=True)
        shutil.copytree(staged, output)
        return {'components': len(entries), 'unique_texts': len(text_count), 'review_gaps': len(gaps), 'complete': False, 'inventory_sha256': digest((output / 'inventory.json').read_bytes())}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--cargo-about', type=pathlib.Path, required=True)
    parser.add_argument('--out', type=pathlib.Path, required=True)
    args = parser.parse_args()
    print(json.dumps(collect(args.cargo_about.resolve(), args.out.resolve())))
