import { screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { it, expect } from 'vitest';
import { fakeApi, renderWithApp } from '../test/render';
import { AgentSubmission } from './AgentSubmission';
import { SubmitForm } from '../pages/Submit';
import { author } from '../test/fixtures';

it('copies a complete author-controlled submission instruction without credentials', async () => {
  const user = userEvent.setup();
  renderWithApp(<AgentSubmission compact />, fakeApi());
  await user.click(screen.getByText('Submit with your agent'));
  expect(screen.getByRole('link', { name: 'Read the agent submission guide' })).toHaveAttribute(
    'href',
    '/skill.md',
  );
  await user.click(screen.getByRole('button', { name: 'Copy submission instruction' }));
  expect(await navigator.clipboard.readText()).toContain('inspect the compiled PDF');
  expect(await navigator.clipboard.readText()).toContain('http://localhost:3000/skill.md');
  expect(await navigator.clipboard.readText()).not.toContain('access_token');
});
it('prefills an editable original paper source when proposing a further question', async () => {
  renderWithApp(
    <SubmitForm initialOrigin="https://example.test/papers/doi/10.1234/paper" />,
    fakeApi({}, author),
  );
  expect(screen.getByLabelText('Original source (optional)')).toHaveValue(
    'https://example.test/papers/doi/10.1234/paper',
  );
});
