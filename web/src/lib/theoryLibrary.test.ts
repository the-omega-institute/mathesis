import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import {
  areaModules,
  findTheoryArea,
  formalLibrary,
  libraryGroups,
  moduleChildren,
  moduleSource,
  researchAreas,
  theoryAreas,
} from './theoryLibrary';

describe('the formal theory library', () => {
  it('uses the actual verifier Mathlib pin and covers every catalogue source once', () => {
    const verifier = readFileSync('../infra/verifier/lakefile.toml', 'utf8');
    expect(verifier).toContain(`rev = "${formalLibrary.revision}"`);
    expect(formalLibrary.revision).toMatch(/^[a-f0-9]{40}$/);
    expect(formalLibrary.modules.length).toBeGreaterThan(8000);
    const indexed = theoryAreas.flatMap(areaModules);
    expect(indexed.length).toBe(formalLibrary.modules.length);
    expect(new Set(indexed).size).toBe(formalLibrary.modules.length);
    expect([...indexed].sort()).toEqual(formalLibrary.modules);
    for (const name of indexed) expect(name).toMatch(/^Mathlib(?:\.[A-Za-z0-9_]+)+$/);
    expect(libraryGroups.flatMap((g) => g.areas).sort()).toEqual(
      theoryAreas.map((a) => a.id).sort(),
    );
    for (const area of theoryAreas) {
      for (const id of area.related) expect(findTheoryArea(id)).toBeDefined();
      for (const topic of area.topics) {
        expect(formalLibrary.modules).toContain(topic.module);
        expect(moduleSource(topic.module)).toContain(`/blob/${formalLibrary.revision}/Mathlib/`);
      }
    }
  });
  it('preserves file and directory nodes with the same name without losing descendants', () => {
    const names = [
      'Mathlib.Foo.Bar',
      'Mathlib.Foo.Bar.Basic',
      'Mathlib.Foo.Bar.Baz',
      'Mathlib.Other',
    ];
    expect(moduleChildren('Mathlib.Foo', names)).toEqual(['Bar']);
    expect(moduleChildren('Mathlib.Foo.Bar', names)).toEqual(['Basic', 'Baz']);
    expect(moduleChildren('Mathlib.Foo.Bar.Basic', names)).toEqual([]);
  });
  it('uses supplied MSC first, editorial placement only for known works, and leaves unknown work unclassified', () => {
    expect(researchAreas('kmvq31-sl3-obstruction')).toEqual(['algebra', 'linear-algebra']);
    expect(researchAreas('kmvq31-sl3-obstruction', ['54A10'])).toEqual(['topology']);
    expect(researchAreas('new-work')).toEqual([]);
    expect(researchAreas('new-work', ['not-a-code'])).toEqual([]);
  });
});
