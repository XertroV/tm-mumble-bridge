"""Validate, package and publish releases using the standard library and gh CLI."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tarfile
import tempfile
import tomllib
import zipfile

PLATFORMS = {"linux-x86_64": "tar.gz", "windows-x86_64": "zip"}
DOCS = ("README.md", "LICENSE", "RELEASE.md")


def metadata(root=Path('.'), tag=None, require_notes=False):
    """Reject inconsistent versions and require reviewed notes when publishing."""
    manifest = tomllib.loads((root / 'Cargo.toml').read_text(encoding='utf-8'))
    version = manifest['package']['version']
    if not re.fullmatch(r'(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)', version):
        raise ValueError('Only stable X.Y.Z versions are supported')
    lock = tomllib.loads((root / 'Cargo.lock').read_text(encoding='utf-8'))
    packages = [p for p in lock['package'] if p['name'] == manifest['package']['name']]
    if len(packages) != 1 or packages[0]['version'] != version:
        raise ValueError('Cargo.toml and Cargo.lock package versions differ')
    if tag is not None and tag != f'v{version}':
        raise ValueError(f'Tag {tag!r} does not match v{version}')
    notes = root / 'releases' / f'v{version}.md'
    if require_notes:
        text = notes.read_text(encoding='utf-8').strip()
        if not text.startswith(f'# v{version}\n') or not text.partition('\n')[2].strip():
            raise ValueError('Release notes need a matching version heading and changelog')
    return version, notes


def preparation(root, before, after):
    """Only a matching version-bump push is an automatic release request."""
    for sha in (before, after):
        if not re.fullmatch(r"[0-9a-f]{40}", sha) or sha == "0" * 40:
            raise ValueError("Release preparation needs existing before/after commits")
    version, notes = metadata(root)
    changed = set(subprocess.check_output(
        ['git', 'diff', '--name-only', before, after], cwd=root, text=True
    ).splitlines())
    required = {'Cargo.toml', 'Cargo.lock', f'releases/v{version}.md'}
    if not required <= changed:
        return None
    def old_toml(path):
        return tomllib.loads(subprocess.check_output(
            ['git', 'show', f'{before}:{path}'], cwd=root, text=True
        ))
    old_version = old_toml('Cargo.toml')['package']['version']
    old_packages = [p for p in old_toml('Cargo.lock')['package'] if p['name'] == 'tm-mumble-link']
    if len(old_packages) != 1 or old_packages[0]['version'] != old_version:
        raise ValueError('Previous Cargo package versions differ')
    if tuple(map(int, version.split('.'))) <= tuple(map(int, old_version.split('.'))):
        return None
    metadata(root, require_notes=True)
    return version


def archive_name(version, platform):
    return f'tm-mumble-link-v{version}-{platform}.{PLATFORMS[platform]}'


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def package(root, binary, platform, tui_binary=None):
    version, _ = metadata(root)
    if not binary.is_file() or binary.stat().st_size == 0:
        raise ValueError('Release binary is missing or empty')
    out = root / 'dist'
    out.mkdir(exist_ok=True)
    archive = out / archive_name(version, platform)
    executable = 'tm-mumble-link.exe' if platform.startswith('windows') else 'tm-mumble-link'
    executable_names = {executable}
    files = [(binary, executable)]
    if tui_binary is not None:
        tui_name = 'tm-mumble-link-tui.exe' if platform.startswith('windows') else 'tm-mumble-link-tui'
        if not tui_binary.is_file() or tui_binary.stat().st_size == 0:
            raise ValueError('Release TUI binary is missing or empty')
        files.append((tui_binary, tui_name))
        executable_names.add(tui_name)
    files += [(root / name, name) for name in DOCS]
    if platform.startswith('linux'):
        with tarfile.open(archive, 'w:gz') as bundle:
            for source, name in files:
                info = bundle.gettarinfo(str(source), arcname=name)
                info.mode = 0o755 if name in executable_names else 0o644
                with source.open('rb') as stream:
                    bundle.addfile(info, stream)
    else:
        with zipfile.ZipFile(archive, 'w', zipfile.ZIP_DEFLATED) as bundle:
            for source, name in files:
                bundle.write(source, arcname=name)
    archive.with_name(archive.name + '.sha256').write_text(
        f'{digest(archive)}  {archive.name}\n', encoding='utf-8'
    )
    return archive


def verify_assets(directory, version):
    """Require exactly both platform archives and valid SHA-256 sidecars."""
    expected = {archive_name(version, p) for p in PLATFORMS}
    expected |= {name + '.sha256' for name in list(expected)}
    if {p.name for p in directory.iterdir()} != expected:
        raise ValueError(f'Expected exactly these assets: {sorted(expected)}')
    for platform in PLATFORMS:
        path = directory / archive_name(version, platform)
        if path.stat().st_size == 0:
            raise ValueError('Empty archive')
        required = {'tm-mumble-link', 'tm-mumble-link-tui'}
        if platform.startswith('windows'):
            required = {name + '.exe' for name in required}
        try:
            if platform.startswith('linux'):
                with tarfile.open(path) as bundle:
                    members = {member.name for member in bundle.getmembers()}
            else:
                with zipfile.ZipFile(path) as bundle:
                    members = set(bundle.namelist())
        except (tarfile.TarError, zipfile.BadZipFile, OSError) as error:
            raise ValueError(f'Invalid archive: {path.name}') from error
        if not required <= members:
            raise ValueError(f'Missing executable(s) in {path.name}')
        sidecar = path.with_name(path.name + '.sha256').read_text(encoding='utf-8')
        if sidecar != f'{digest(path)}  {path.name}\n':
            raise ValueError(f'Checksum mismatch: {path.name}')
    return sorted(directory.iterdir())


def gh(*args):
    return subprocess.check_output(['gh', *args], text=True, encoding='utf-8')


def find_release(tag):
    # GET /releases/tags/{tag} only finds published releases. The authenticated
    # list endpoint also returns drafts, including drafts from failed attempts.
    page = 1
    while True:
        releases = json.loads(gh(
            'api', f'repos/{os.environ["GH_REPO"]}/releases?per_page=100&page={page}'
        ))
        for release in releases:
            if release['tag_name'] == tag:
                return release
        if len(releases) < 100:
            return None
        page += 1


def verify_download(tag, version, assets, release, compare_build=True):
    expected = {p.name for p in assets}
    if {a['name'] for a in release['assets']} != expected:
        raise ValueError('Release does not contain exactly the expected assets')
    if any(a['state'] != 'uploaded' or a['size'] == 0 for a in release['assets']):
        raise ValueError('Release contains an incomplete upload')
    with tempfile.TemporaryDirectory() as tmp:
        gh('release', 'download', tag, '--dir', tmp)
        downloaded = Path(tmp)
        verify_assets(downloaded, version)
        for asset in assets:
            if compare_build and digest(asset) != digest(downloaded / asset.name):
                raise ValueError(f'Download differs from build artifact: {asset.name}')


def publish(root):
    tag = os.environ['RELEASE_TAG']
    version, notes = metadata(root, tag, require_notes=True)
    assets = verify_assets(root / 'dist', version)
    head = subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip()
    tagged = subprocess.check_output(['git', 'rev-parse', f'{tag}^{{commit}}'], text=True).strip()
    if head != tagged:
        raise ValueError('Refusing to publish artifacts for a different commit')
    release = find_release(tag)
    if release and not release['draft']:
        # Published releases are immutable: verify a rerun instead of replacing files.
        verify_download(tag, version, assets, release, compare_build=False)
        if release["body"].strip() != notes.read_text(encoding="utf-8").strip():
            raise ValueError("Published release notes differ from the tagged changelog")
        print(f'Already published and verified: {release["html_url"]}')
        return
    if release is None:
        gh('release', 'create', tag, '--verify-tag', '--draft', '--title', tag,
           '--notes-file', str(notes))
    else:
        gh('release', 'edit', tag, '--title', tag, '--notes-file', str(notes))
    gh('release', 'upload', tag, *map(str, assets), '--clobber')
    release = find_release(tag)
    verify_download(tag, version, assets, release)
    # Uploads are verified while still a draft; users never see a partial release.
    gh('release', 'edit', tag, '--draft=false', '--latest')
    release = find_release(tag)
    if release['draft'] or release['body'].strip() != notes.read_text(encoding='utf-8').strip():
        raise ValueError('Release publication or changelog verification failed')
    verify_download(tag, version, assets, release)
    print(f'Published and verified: {release["html_url"]}')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='command', required=True)
    meta = sub.add_parser('metadata')
    meta.add_argument('--require-notes', action='store_true')
    pack = sub.add_parser('package')
    pack.add_argument('--platform', choices=PLATFORMS, required=True)
    pack.add_argument('--binary', type=Path, required=True)
    pack.add_argument('--tui-binary', type=Path)
    sub.add_parser('publish')
    sub.add_parser('prepare')
    args = parser.parse_args()
    root = Path('.')
    if args.command == 'metadata':
        version, _ = metadata(root, os.environ.get('RELEASE_TAG'), args.require_notes)
        output = f'version={version}\ntag=v{version}\n'
        print(output, end='')
        if os.environ.get('GITHUB_OUTPUT'):
            with open(os.environ['GITHUB_OUTPUT'], 'a', encoding='utf-8') as stream:
                stream.write(output)
    elif args.command == 'prepare':
        version = preparation(root, os.environ['BEFORE_SHA'], os.environ['AFTER_SHA'])
        output = 'ready=false\n' if version is None else f'ready=true\nversion={version}\ntag=v{version}\n'
        print(output, end='')
        if os.environ.get('GITHUB_OUTPUT'):
            with open(os.environ['GITHUB_OUTPUT'], 'a', encoding='utf-8') as stream:
                stream.write(output)
    elif args.command == 'package':
        print(package(root, args.binary, args.platform, args.tui_binary))
    else:
        publish(root)


if __name__ == '__main__':
    main()
