import { expect, it } from 'vitest';
import { texToText } from './texText';

it('turns TeX accents and ligatures in names into text', () => {
  expect(texToText('Gustav Kj\\ae rbye Bagger')).toBe('Gustav Kjærbye Bagger');
  expect(texToText("Paul Erd\\H{o}s and Andr\\'e Weil")).toBe('Paul Erdős and André Weil');
  expect(texToText('{\\"O}zlem {\\c{C}}elik')).toBe('Özlem Çelik');
  expect(texToText('L\\o ve~Olsen and Ka\\v{s}par')).toBe('Løve Olsen and Kašpar');
  expect(texToText('Plain Name')).toBe('Plain Name');
});
