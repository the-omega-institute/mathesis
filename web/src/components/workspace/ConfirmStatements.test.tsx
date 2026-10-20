import { fireEvent, screen, waitFor } from '@testing-library/react';
import { expect, it, vi } from 'vitest';
import { draft, extracted } from '../../test/fixtures';
import { fakeApi, renderWithApp } from '../../test/render';
import { ConfirmStatements } from './ConfirmStatements';

it('accepts extracted defaults and submits hidden rows without manual checking', async () => {
  const claims = Array.from({ length: 41 }, (_, i) => ({
    ...extracted[0]!,
    id: `C${i + 1}`,
    label: `Theorem ${i + 1}`,
    role: 'main' as const,
  }));
  const paper = { ...draft, extracted: claims, claims: [] };
  const confirmClaims = vi.fn(async () => paper);
  renderWithApp(
    <ConfirmStatements submission={paper} onChange={vi.fn()} />,
    fakeApi({ confirmClaims }),
  );
  expect(screen.getByRole('button', { name: 'Start review with these statements' })).toBeEnabled();
  fireEvent.change(screen.getByLabelText('Role of C1'), { target: { value: 'supporting' } });
  for (let page = 0; page < 9; page++) {
    if (page < 8) fireEvent.click(screen.getByRole('button', { name: 'Next statements' }));
  }
  expect(screen.getByText('41 statements detected')).toBeInTheDocument();
  fireEvent.change(screen.getByLabelText('Jump to statement'), { target: { value: 'C1' } });
  expect(screen.getByLabelText('Role of C1')).toHaveValue('supporting');
  fireEvent.change(screen.getByLabelText('Find a statement'), { target: { value: 'Theorem 41' } });
  fireEvent.click(screen.getByRole('button', { name: 'Start review with these statements' }));
  await waitFor(() => expect(confirmClaims).toHaveBeenCalled());
  const rows = confirmClaims.mock.calls[0] as unknown as [string, { id: string; role: string }[]];
  expect(rows[1]).toHaveLength(41);
  expect(rows[1][0]).toMatchObject({ id: 'C1', role: 'supporting' });
}, 15000);
