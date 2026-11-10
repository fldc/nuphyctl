#!/usr/bin/env python3
"""Collect original redistribution notices for a locked, target-specific build."""
import argparse
import hashlib
import json
import re
import shutil
import subprocess
import tarfile
import tomllib
from pathlib import Path


def run(*args):
    return subprocess.check_output(args, text=True).strip()


def digest(data):
    return hashlib.sha256(data).hexdigest()


def collect(manifest, target, binary, output, rust_version):
    manifest = manifest.resolve()
    binary = binary.resolve()
    lock_bytes = manifest.with_name('Cargo.lock').read_bytes()
    lock = tomllib.loads(lock_bytes.decode())
    locked = {(p['name'], p['version']): p for p in lock['package']}
    actual_rust = run('rustc', '--version', '--verbose')
    if f'release: {rust_version}' not in actual_rust.splitlines():
        raise ValueError('Rust toolchain does not match the binary build record')
    metadata = json.loads(run('cargo', 'metadata', '--locked', '--format-version', '1',
                              '--filter-platform', target, '--manifest-path', str(manifest)))
    packages = {p['id']: p for p in metadata['packages']}
    nodes = {n['id']: n for n in metadata['resolve']['nodes']}
    root = metadata['resolve']['root']
    selected = set()
    runtime = set()

    def visit(pid, linked):
        seen = runtime if linked else selected
        if pid in seen:
            return
        seen.add(pid)
        selected.add(pid)
        for dep in nodes[pid]['deps']:
            kinds = {k['kind'] for k in dep['dep_kinds']}
            if kinds == {'dev'}:
                continue
            proc_macro = any('proc-macro' in t['kind'] for t in packages[dep['pkg']]['targets'])
            visit(dep['pkg'], linked and None in kinds and not proc_macro)

    visit(root, True)
    if output.exists():
        raise ValueError(f'Output already exists: {output}; use a fresh directory')
    output.mkdir(parents=True)
    records = []
    for pid in sorted(selected, key=lambda i: (packages[i]['name'], packages[i]['version'])):
        p = packages[pid]
        if pid == root:
            continue
        if not (p.get('source') or '').startswith('registry+'):
            raise ValueError(f'Unsupported non-registry dependency: {pid}')
        source = Path(p['manifest_path']).parent
        archive = source.parents[2] / 'cache' / source.parent.name / f"{p['name']}-{p['version']}.crate"
        checksum = locked[(p['name'], p['version'])]['checksum']
        if digest(archive.read_bytes()) != checksum:
            raise ValueError(f'Crate checksum mismatch: {archive}')
        destination = output / 'crates' / f"{p['name']}-{p['version']}"
        texts = []
        with tarfile.open(archive) as tf:
            for member in tf.getmembers():
                relative = Path(*Path(member.name).parts[1:])
                if not member.isfile() or relative.is_absolute() or '..' in relative.parts:
                    continue
                notice = re.match(r'^(license|licence|copying|copyright|notice|unlicense)([.\-_]|$)', relative.name, re.I)
                # Root crate notices plus nested native sources' original notices.
                if not notice:
                    continue
                data = tf.extractfile(member).read()
                path = destination / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(data)
                texts.append({'path': str(path.relative_to(output)), 'sha256': digest(data)})
            if p['name'] == 'hidapi':
                # Preserve the native implementation's attribution header as well.
                name = f"{p['name']}-{p['version']}/etc/hidapi/linux/hid.c"
                data = tf.extractfile(name).read().split(b'*/', 1)[0] + b'*/\n'
                (destination / 'native-linux-header.txt').write_bytes(data)
                texts.append({'path': str((destination / 'native-linux-header.txt').relative_to(output)), 'sha256': digest(data)})
                native_version = tf.extractfile(f"{p['name']}-{p['version']}/etc/hidapi/VERSION").read().decode().strip()
        if not texts:
            raise ValueError(f'No original license texts found for {pid}')
        record = {'name': p['name'], 'version': p['version'], 'license_expression': p['license'],
                  'role': 'runtime dependency (may be optimized out)' if pid in runtime else 'build/proc-macro dependency (conservative inclusion)',
                  'features': sorted(nodes[pid]['features']), 'registry_checksum': checksum,
                  'repository': p.get('repository'), 'notices': texts}
        if p['name'] == 'hidapi':
            record['native_component'] = {'name': 'HIDAPI C implementation', 'version': native_version,
                                          'backend': 'linux/hid.c (linux-static-hidraw)',
                                          'license_choice': 'BSD-style (LICENSE-bsd.txt); alternative texts retained'}
        records.append(record)
    rust_docs = Path(run('rustc', '--print', 'sysroot')) / 'share/doc/rust'
    rust_dir = output / 'rust'
    rust_dir.mkdir()
    shutil.copyfile(rust_docs / 'COPYRIGHT-library.html', rust_dir / 'COPYRIGHT-library.html')
    shutil.copytree(rust_docs / 'licenses', rust_dir / 'licenses')
    (rust_dir / 'rustc-version.txt').write_text(actual_rust + '\n')
    dynamic = run('readelf', '-d', str(binary))
    needed = re.findall(r'Shared library: \[(.*?)\]', dynamic)
    result = {'schema_version': 1, 'package': packages[root]['name'], 'version': packages[root]['version'],
              'source_commit': run('git', '-C', str(manifest.parent), 'rev-parse', 'HEAD'),
              'cargo_lock_sha256': digest(lock_bytes), 'target': target, 'cargo_features': sorted(nodes[root]['features']),
              'binary_sha256': digest(binary.read_bytes()), 'rust_version': rust_version,
              'rust_notices': 'rust/COPYRIGHT-library.html', 'dynamic_system_libraries': needed,
              'components': records}
    (output / 'components.json').write_text(json.dumps(result, indent=2, sort_keys=True) + '\n')
    (output / 'README.md').write_text(f'''# Third-party notices: nuphyctl {result['version']}

Target: `{target}`. Executable SHA-256: `{result['binary_sha256']}`.
Source commit: `{result['source_commit']}`.

`components.json` maps the locked crate versions, enabled features, registry
checksums, roles, and original notice paths for this target's default-feature
release build. Runtime entries describe the Cargo dependency graph, not an
object-level assertion that every function survives optimization. Build helpers
and procedural macros are conservatively included and separately identified.
Windows-only crates are excluded on these Linux targets.

Original crate texts are copied without modification from registry archives
verified against Cargo.lock. Dual-license alternatives are retained. Rust {rust_version}
standard-library and compiler-builtins attributions, including embedded third-party
components, are in `rust/COPYRIGHT-library.html` and `rust/licenses/` from the
matching toolchain distribution. That upstream mapping is conservative across
standard-library configurations; the Rust compiler itself is not shipped here.

The native HIDAPI implementation is statically compiled using linux-static-hidraw;
its BSD-style license is selected and the alternative original texts are retained.
The dynamically required system libraries listed in components.json are supplied
by the operating system, not bundled in this archive. Their installed distribution
packages provide their own notices.

These notices supplement nuphyctl's own MIT LICENSE. They do not change upstream
license terms or establish reproducible-build equivalence.
''')
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--manifest-path', type=Path, default=Path('Cargo.toml'))
    parser.add_argument('--target', required=True, choices=['x86_64-unknown-linux-gnu', 'aarch64-unknown-linux-gnu'])
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--rust-version', required=True)
    args = parser.parse_args()
    result = collect(args.manifest_path, args.target, args.binary, args.output, args.rust_version)
    print(f"Collected {len(result['components'])} components for {args.target}")


if __name__ == '__main__':
    main()
