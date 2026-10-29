import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, it, vi } from 'vitest';
import { author } from '../test/fixtures';
import { fakeApi, renderWithApp } from '../test/render';
import type { Person } from '../api/types';
import { PeoplePage } from './People';

it('offers only administrator access and preserves historical roles when saving', async () => {
  const admin: Person = { ...author, roles: ['admin'] };
  const account: Person = { ...author, id: 'other', roles: ['editor', 'endorser'] };
  const setRoles = vi.fn(async () => ({
    ...account,
    roles: ['editor', 'endorser', 'admin'] as Person['roles'],
  }));
  renderWithApp(
    <PeoplePage />,
    fakeApi(
      { listPeople: vi.fn(async () => ({ items: [account], next_before: null })), setRoles },
      admin,
    ),
  );
  const checkbox = await screen.findByRole('checkbox', { name: 'Administrator' });
  expect(screen.getAllByRole('checkbox')).toHaveLength(1);
  expect(screen.queryByText('Editor')).toBeNull();
  expect(screen.queryByText('Reviewer')).toBeNull();
  expect(screen.queryByText('Endorser')).toBeNull();
  const user = userEvent.setup();
  await user.click(checkbox);
  await user.click(screen.getByRole('button', { name: 'Save access' }));
  expect(setRoles).toHaveBeenCalledWith('other', ['editor', 'endorser', 'admin']);
});
