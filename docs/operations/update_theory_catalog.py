#!/usr/bin/env python3
"""Consume a complete GitHub recursive tree for the existing verifier Mathlib pin."""
import argparse
import json
from pathlib import Path
import re


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--tree', type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    verifier = (root / 'infra/verifier/lakefile.toml').read_text()
    revision = re.search(r'^rev = "([a-f0-9]{40})"$', verifier, re.MULTILINE).group(1)
    tree = json.loads(args.tree.read_text())
    if tree.get('sha') != revision or tree.get('truncated') is not False:
        raise ValueError('Need the complete recursive tree for the current exact Mathlib pin')
    modules = sorted(row['path'][:-5].replace('/', '.') for row in tree['tree']
                     if row['type'] == 'blob' and row['path'].startswith('Mathlib/')
                     and row['path'].endswith('.lean'))
    if not modules or len(set(modules)) != len(modules) or any(
            not re.fullmatch(r'Mathlib(?:\.[A-Za-z0-9_]+)+', name) for name in modules):
        raise ValueError('Invalid or duplicate module identities')
    areas = json.loads((root / 'web/src/data/theory-areas.json').read_text())
    roots = [part for area in areas for part in area['roots']]
    if len(roots) != len(set(roots)) or set(roots) != {name.split('.')[1] for name in modules}:
        raise ValueError('Reading areas must cover every source root exactly once')
    if any(topic['module'] not in modules for area in areas for topic in area['topics']):
        raise ValueError('A reading topic points outside this source version')
    output = root / 'web/src/data/mathlib-catalog.json'
    output.write_text(json.dumps({'repository': 'https://github.com/leanprover-community/mathlib4',
                                 'revision': revision, 'license': 'Apache-2.0', 'modules': modules},
                                ensure_ascii=False, indent=2) + '\n')
    print(f'{len(modules)} source paths at {revision}')


if __name__ == '__main__':
    main()
