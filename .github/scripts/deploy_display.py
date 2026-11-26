#!/usr/bin/env python3
"""Deploy an exact CI-published Mathesis Web image through the scoped CD route."""
import argparse
import copy
import io
import json
import os
from pathlib import Path
import re
import subprocess
import time
import urllib.request
import zipfile

REPOSITORY = 'the-omega-institute/mathesis'
IMAGE = 'ghcr.io/the-omega-institute/mathesis-web@sha256:'
CD = 'https://api.nyxid.ai/api/v1/proxy/s/mathesis-display-deploy/api/v1/projects/omega/apps/mathesis'
ORG = '58116a9d-3426-4b90-9e25-2154e34788e9'
PUBLIC = 'https://mathesis-omega-research.apps.chrono-ai.fun/'
OID = re.compile(r'[0-9a-f]{40}')


def require(condition, message):
    if not condition:
        raise ValueError(message)


def github(path):
    return json.loads(subprocess.check_output(['gh', 'api', 'repos/' + REPOSITORY + '/' + path], text=True))


def prepare(run_id):
    require(str(run_id).isdigit(), 'positive image workflow run ID required')
    run = github('actions/runs/' + str(run_id))
    require(run['path'] == '.github/workflows/publish-images.yml'
            and run['status'] == 'completed' and run['conclusion'] == 'success', 'image publication has not succeeded')
    assets = github('actions/runs/' + str(run_id) + '/artifacts?per_page=100')['artifacts']
    assets = [a for a in assets if a['name'].startswith('mathesis-app-image-receipts-') and not a['expired']]
    require(len(assets) == 1, 'one live immutable app image receipt is required')
    raw = subprocess.check_output(['gh', 'api', 'repos/' + REPOSITORY + '/actions/artifacts/' + str(assets[0]['id']) + '/zip'])
    with zipfile.ZipFile(io.BytesIO(raw)) as archive:
        candidates = [n for n in archive.namelist() if n == 'web.json']
        require(candidates == ['web.json'], 'unexpected Web receipt inventory')
        receipt = json.loads(archive.read('web.json'))
    validate_receipt(receipt, run_id)
    source = receipt['source_commit']
    require(github('git/ref/heads/develop')['object']['sha'] == source, 'newer default-branch work exists; refuse stale deployment')
    runs = github('actions/workflows/ci.yml/runs?head_sha=' + source + '&event=push&status=success&per_page=100')['workflow_runs']
    require(any(r['head_sha'] == source and r['head_branch'] == 'develop' for r in runs), 'successful push CI for the exact source is required')
    return dict(receipt, image_run_id=str(run_id))


def validate_receipt(receipt, run_id):
    require(receipt.get('source_ref') == 'develop' and OID.fullmatch(receipt.get('source_commit', '')), 'invalid source identity')
    require(str(receipt.get('workflow_run')) == str(run_id), 'foreign image workflow receipt')
    require(re.fullmatch(re.escape(IMAGE) + '[0-9a-f]{64}', receipt.get('image', '')), 'foreign or mutable Web image')


def cd(method='GET', spec=None, generation=None, status=False):
    url = CD + ('/status' if status else '') + '?org_id=' + ORG
    headers = {'Authorization': 'Bearer ' + os.environ['NYXID_MATHESIS_DEPLOY_KEY'],
               'Content-Type': 'application/json', 'User-Agent': 'mathesis-display-rollout'}
    if generation is not None:
        headers['If-Match'] = str(generation)
    body = json.dumps(spec).encode() if spec is not None else None
    with urllib.request.urlopen(urllib.request.Request(url, data=body, headers=headers, method=method), timeout=60) as response:
        return json.load(response)


def image_revision(image):
    require(re.fullmatch(re.escape(IMAGE) + '[0-9a-f]{64}', image), 'current Web image is outside the registered repository')
    # The existing image publisher retains immutable source/image receipts for
    # 90 days. Use those records rather than exposing a registry credential.
    runs = github('actions/workflows/publish-images.yml/runs?status=success&per_page=50')['workflow_runs']
    for run in runs:
        if run['path'] != '.github/workflows/publish-images.yml':
            continue
        assets = github('actions/runs/' + str(run['id']) + '/artifacts?per_page=100')['artifacts']
        for asset in assets:
            if not asset['name'].startswith('mathesis-app-image-receipts-') or asset['expired']:
                continue
            raw = subprocess.check_output(['gh', 'api', 'repos/' + REPOSITORY + '/actions/artifacts/' + str(asset['id']) + '/zip'])
            with zipfile.ZipFile(io.BytesIO(raw)) as archive:
                receipt = json.loads(archive.read('web.json'))
            if receipt['image'] == image:
                require(OID.fullmatch(receipt.get('source_commit', ''))
                        and str(receipt.get('workflow_run')) == str(run['id']), 'deployed image receipt is not source-bound')
                return receipt['source_commit']
    raise ValueError('deployed Web digest has no retained publisher receipt; reconcile its original source before rollout')


def validate_changes(files):
    operational = {'.github/workflows/display-rollout.yml', '.github/workflows/display-monitor.yml',
                   '.github/display-monitor.json', '.github/scripts/deploy_display.py', '.github/scripts/test_deploy_display.py'}
    require(all(f['filename'].startswith('web/src/') or f['filename'] in operational for f in files),
            'source delta includes changes outside display maintenance; use the existing operator deployment path')


def desired_spec(before, image):
    require(before['project'] == 'omega' and before['name'] == 'mathesis', 'foreign CD application')
    spec = copy.deepcopy(before['spec'])
    spec['services']['web']['image'] = image
    for service in spec['services'].values():
        service.pop('image_digest', None)  # CD returns this computed field on reads.
    return spec


def web_ready(status, observed, image):
    web = status.get('services', {}).get('web', {})
    replicas = observed['spec']['services']['web'].get('replicas', 1)
    pods = [p for p in web.get('pods', []) if p.get('ready')]
    reconciled = status.get('last_reconcile', {})
    return (reconciled.get('ok') is True and reconciled.get('revision') == observed['revision']
            and web.get('ready', 0) >= replicas and web.get('updated', 0) >= replicas
            and len(pods) >= replicas
            and all(p.get('app_container', {}).get('image_id') == image for p in pods))


def apply(plan, dry_run=True):
    # Revalidate trusted GitHub provenance immediately before exposing the CD key.
    require(prepare(plan['image_run_id']) == plan, 'image/source plan changed')
    before = cd()
    current = before['spec']['services']['web']['image']
    baseline = image_revision(current)
    comparison = github('compare/' + baseline + '...' + plan['source_commit'])
    require(comparison['status'] in ('ahead', 'identical'), 'source is not a descendant of the deployed Web version')
    require(len(comparison.get('files', [])) < 300, 'source delta exceeds API coverage')
    validate_changes(comparison.get('files', []))
    if dry_run:
        return {'state':'validated', 'image':plan['image'], 'source_commit':plan['source_commit'], 'generation':before['generation']}
    if current != plan['image']:
        # One optimistic write only. A lost response is reconciled by the same
        # image on the next run; it never triggers a blind write retry.
        cd('PUT', desired_spec(before, plan['image']), before['generation'])
    for _ in range(24):
        observed = cd()
        require(observed['spec']['services']['web']['image'] == plan['image'], 'a concurrent operator changed the Web image')
        for name, service in before['spec']['services'].items():
            if name != 'web':
                require(observed['spec']['services'][name] == service, 'another service changed during Web rollout')
        status = cd(status=True)
        if web_ready(status, observed, plan['image']):
            with urllib.request.urlopen(PUBLIC, timeout=30) as response:
                require(response.status == 200 and b'<html' in response.read(1024*1024).lower(), 'public Web readback failed')
            return {'state':'deployed', 'image':plan['image'], 'source_commit':plan['source_commit'], 'generation':observed['generation']}
        time.sleep(5)
    raise ValueError('Web rollout has not become ready; preserve the image and inspect the original deployment')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['prepare','apply'])
    parser.add_argument('--run-id')
    parser.add_argument('--plan', required=True)
    parser.add_argument('--dry-run', action='store_true')
    args = parser.parse_args()
    if args.action == 'prepare':
        Path(args.plan).write_text(json.dumps(prepare(args.run_id)))
    else:
        print(json.dumps(apply(json.loads(Path(args.plan).read_text()), args.dry_run)))
