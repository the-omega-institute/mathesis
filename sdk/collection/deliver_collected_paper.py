#!/usr/bin/env python3
"""Package a repository and deliver an immutable trusted collection version.

No third-party dependencies. Dry runs are offline unless --auto-previous-version
is explicitly requested. The server remains the authorization/validation authority.
"""
import argparse
import copy
import hashlib
import io
import json
import os
from pathlib import Path
import re
import sys
from urllib.error import HTTPError, URLError
from urllib.parse import urlsplit
from urllib.request import HTTPRedirectHandler, Request, build_opener
import zipfile

MAX_FILE = 30 * 1024 * 1024
SCHEMA = 'mathesis.pre-reviewed-delivery.v1'


class NoRedirect(HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None


def request(url, *, token=None, data=None, content_type=None):
    headers = {'Accept': 'application/json', 'User-Agent': 'mathesis-collection/1'}
    if token:
        headers['Authorization'] = f'Bearer {token}'
    if content_type:
        headers['Content-Type'] = content_type
    try:
        with build_opener(NoRedirect).open(Request(url, data=data, headers=headers), timeout=60) as response:
            return json.load(response)
    except HTTPError as error:
        if error.code == 404 and data is None:
            return None
        # Do not print response bodies or request headers containing private data.
        raise ValueError(f'Mathesis returned HTTP {error.code}') from None
    except URLError:
        raise ValueError('Mathesis request failed; check URL and connectivity') from None


def api_base(raw):
    parsed = urlsplit(raw)
    local = parsed.hostname in ('localhost', '127.0.0.1', '::1')
    if (parsed.scheme != 'https' and not (parsed.scheme == 'http' and local)) or not parsed.hostname or parsed.username or parsed.password or parsed.query or parsed.fragment:
        raise ValueError('MATHESIS_URL requires HTTPS (HTTP is allowed on loopback only)')
    base = raw.rstrip('/')
    return base if base.endswith('/api/v1') else base + '/api/v1'


def inside(repo, name):
    path = repo / name
    if path.is_symlink() or not path.resolve().is_relative_to(repo):
        raise ValueError('input paths must stay inside the paper repository without symlinks')
    return path


def deterministic_zip(source):
    """Stable paths, order, timestamp, mode and uncompressed bytes across platforms."""
    if not source.is_dir():
        raise ValueError('source directory does not exist')
    output = io.BytesIO()
    files = sorted(source.rglob('*'), key=lambda p: p.relative_to(source).as_posix())
    count = 0
    total = 0
    with zipfile.ZipFile(output, 'w', compression=zipfile.ZIP_STORED) as archive:
        for path in files:
            if path.is_symlink():
                raise ValueError('source archive refuses symlinks')
            if path.is_dir():
                continue
            if not path.is_file():
                raise ValueError('source archive accepts regular files only')
            name = path.relative_to(source).as_posix()
            if any(part in ('.git', '.env', 'node_modules', '__pycache__') for part in path.relative_to(source).parts):
                raise ValueError('source directory contains private or generated files; stage manuscript inputs only')
            if '\\' in name or any(ord(c) < 32 for c in name):
                raise ValueError('invalid source path')
            total += path.stat().st_size
            count += 1
            if total > MAX_FILE or count > 2000:
                raise ValueError('source exceeds 30 MiB or 2000 files')
            info = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
            info.create_system = 3
            info.external_attr = 0o100644 << 16
            archive.writestr(info, path.read_bytes())
    if count == 0 or len(output.getvalue()) > MAX_FILE:
        raise ValueError('source ZIP must be nonempty and at most 30 MiB')
    return output.getvalue()


def text(value, name, limit):
    if not isinstance(value, str) or not value.strip() or len(value.strip()) > limit:
        raise ValueError(f'{name} must be nonempty and at most {limit} characters')


def github(url, issues=False):
    owner = r'[A-Za-z0-9](?:[A-Za-z0-9-]{0,37}[A-Za-z0-9])?'
    repo = r'[A-Za-z0-9_.-]{1,100}'
    match = re.fullmatch(rf'https://github\.com/({owner})/({repo})' + ('/issues' if issues else ''), url)
    if not match or match[2] in ('.', '..') or '--' in match[1]:
        raise ValueError('require a public HTTPS GitHub repository or /issues URL')
    return url.removesuffix('/issues') if issues else url


def validate(envelope):
    if envelope.get('schema') != SCHEMA or envelope.get('authorize_publication') is not True:
        raise ValueError('require delivery v1 schema and explicit authorize_publication: true')
    if set(envelope) - {'schema', 'producer', 'work_id', 'version', 'previous_version', 'authorize_publication', 'paper', 'review'}:
        raise ValueError('unknown envelope field')
    for key in ('producer', 'work_id'):
        if not isinstance(envelope.get(key), str) or not re.fullmatch(r'[a-z0-9-]{1,160}', envelope[key]):
            raise ValueError(f'{key} requires a lowercase ASCII slug')
    version, previous = envelope.get('version'), envelope.get('previous_version')
    if type(version) is not int or not 1 <= version <= 4294967295 or (previous is not None and (type(previous) is not int or not 0 < previous < version)):
        raise ValueError('version must advance its positive predecessor')
    paper = envelope['paper']
    if set(paper) - {'title', 'abstract', 'authors', 'license', 'archive_doi', 'plain_language', 'msc', 'problems', 'lean', 'kind', 'repository_url', 'issues_url', 'version_label', 'changelog'}:
        raise ValueError('unknown paper field')
    text(paper.get('title'), 'title', 1000)
    text(paper.get('abstract'), 'abstract', 20000)
    if paper.get('license') != 'CC-BY-4.0' or (paper.get('kind') if paper.get('kind') is not None else 'note') not in ('paper', 'note', 'methodology'):
        raise ValueError('require CC-BY-4.0 and kind paper, note or methodology')
    authors = paper.get('authors')
    if not isinstance(authors, list) or not authors or len(authors) > 50:
        raise ValueError('supply ordered authors')
    for author in authors:
        text(author.get('name'), 'author name', 200)
        if author.get('person') is not None:
            raise ValueError('do not assert Mathesis account ownership')
    msc = paper.get('msc', [])
    if not isinstance(msc, list) or len(msc) > 8 or any(not isinstance(c, str) or not re.fullmatch(r'[0-9]{2}[A-Za-z0-9-]{0,3}', c) for c in msc):
        raise ValueError('invalid MSC codes')
    repository = paper.get('repository_url')
    if repository is not None:
        github(repository)
    if paper.get('issues_url') is not None:
        issues_repository = github(paper['issues_url'], True)
        if repository and repository != issues_repository:
            raise ValueError('issues_url must belong to repository_url')
    for key, limit in [('version_label', 64), ('changelog', 10000)]:
        if paper.get(key) is not None:
            text(paper[key], key, limit)
            if len(paper[key]) > limit:
                raise ValueError(f'{key} exceeds {limit} characters')
    if paper.get('version_label') and any(ord(c) < 32 or 127 <= ord(c) <= 159 for c in paper['version_label']):
        raise ValueError('version_label contains control characters')
    review = envelope['review']
    if set(review) != {'verdict', 'pdf_sha256', 'source_sha256', 'record'}:
        raise ValueError('invalid review fields')
    if review.get('verdict') not in ('PASS', 'INTERNAL_EDITORIAL') or not isinstance(review.get('record'), dict) or not review['record']:
        raise ValueError('require PASS or INTERNAL_EDITORIAL and nonempty private review record')
    for key in ('pdf_sha256', 'source_sha256'):
        if not re.fullmatch(r'[0-9a-f]{64}', review.get(key, '')):
            raise ValueError('require exact lowercase SHA-256 bindings')
    if len(json.dumps(envelope, ensure_ascii=False).encode()) > 128 * 1024:
        raise ValueError('metadata exceeds 128 KiB')


def package(repo, metadata='mathesis.json', pdf='paper.pdf', source_dir='manuscript', **overrides):
    repo = Path(repo).resolve(strict=True)
    envelope = json.loads(inside(repo, metadata).read_text())
    envelope = copy.deepcopy(envelope)
    for key in ('kind', 'repository_url', 'issues_url', 'version_label', 'changelog'):
        if overrides.get(key) is not None:
            envelope['paper'][key] = overrides[key]
    if overrides.get('verdict') is not None:
        envelope['review']['verdict'] = overrides['verdict']
    pdf_path = inside(repo, pdf)
    if not pdf_path.is_file() or pdf_path.stat().st_size > MAX_FILE:
        raise ValueError('PDF must exist and be at most 30 MiB')
    pdf_bytes = pdf_path.read_bytes()
    if not pdf_bytes.startswith(b'%PDF-'):
        raise ValueError('PDF has no PDF signature')
    source_bytes = deterministic_zip(inside(repo, source_dir))
    for key, value in [('pdf_sha256', pdf_bytes), ('source_sha256', source_bytes)]:
        envelope['review'][key] = hashlib.sha256(value).hexdigest()
    validate(envelope)
    return envelope, pdf_bytes, source_bytes


def previous_version(envelope, base):
    """Resolve predecessor without breaking exact retries of the current version."""
    latest = request(f'{base}/collected-papers/{envelope["work_id"]}')
    if latest is None:
        return None
    current = int(latest['revision'])
    if current > envelope['version']:
        raise ValueError('delivery version is older than the current publication')
    if current == envelope['version']:
        return int(latest['previous_version']) if latest.get('previous_version') else None
    return current


def multipart(envelope, pdf, source):
    metadata = json.dumps(envelope, ensure_ascii=False, separators=(',', ':')).encode()
    boundary = 'mathesis-' + hashlib.sha256(metadata + pdf + source).hexdigest()
    fields = [('metadata', None, 'application/json', metadata), ('pdf', 'paper.pdf', 'application/pdf', pdf), ('source', 'source.zip', 'application/zip', source)]
    body = bytearray()
    for name, filename, content_type, data in fields:
        body.extend(f'--{boundary}\r\nContent-Disposition: form-data; name="{name}"'.encode())
        if filename:
            body.extend(f'; filename="{filename}"'.encode())
        body.extend(f'\r\nContent-Type: {content_type}\r\n\r\n'.encode())
        body.extend(data)
        body.extend(b'\r\n')
    body.extend(f'--{boundary}--\r\n'.encode())
    return bytes(body), f'multipart/form-data; boundary={boundary}'


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('repo', type=Path)
    parser.add_argument('--metadata', default='mathesis.json')
    parser.add_argument('--pdf', default='paper.pdf')
    parser.add_argument('--source-dir', default='manuscript')
    parser.add_argument('--kind', choices=['paper', 'note', 'methodology'])
    parser.add_argument('--verdict', choices=['PASS', 'INTERNAL_EDITORIAL'])
    for flag in ('repository-url', 'issues-url', 'version-label', 'changelog'):
        parser.add_argument('--' + flag)
    parser.add_argument('--dry-run', action='store_true')
    parser.add_argument('--auto-previous-version', action='store_true')
    args = vars(parser.parse_args(argv))
    dry_run = args.pop('dry_run')
    auto_previous = args.pop('auto_previous_version')
    try:
        envelope, pdf, source = package(**args)
        base = api_base(os.environ.get('MATHESIS_URL', '')) if auto_previous or not dry_run else None
        if auto_previous:
            envelope['previous_version'] = previous_version(envelope, base)
            validate(envelope)
        if dry_run:
            print(json.dumps(envelope, ensure_ascii=False, indent=2))
            return 0
        token = os.environ.get('MATHESIS_TOKEN')
        if not token or any(c.isspace() for c in token):
            raise ValueError('set MATHESIS_TOKEN to the trusted operator access token')
        body, content_type = multipart(envelope, pdf, source)
        result = request(base + '/pre-reviewed-deliveries', token=token, data=body, content_type=content_type)
        print(json.dumps(result, ensure_ascii=False, indent=2))
        return 0
    except (ValueError, OSError, KeyError, TypeError) as error:
        # Configuration/validation failures never include environment credentials.
        print(f'Delivery failed: {error}', file=sys.stderr)
        return 1


if __name__ == '__main__':
    sys.exit(main())
