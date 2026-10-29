import { screen } from '@testing-library/react';
import { expect, it } from 'vitest';
import { fakeApi, renderWithApp } from '../test/render';
import { PolicyPage } from './Policy';

it('explains mathematical criteria, decisions, privacy and formalization without internal stages', async () => {
  renderWithApp(<PolicyPage />, fakeApi({}));
  expect(await screen.findByRole('heading', { name: 'Review policy' })).toBeVisible();
  for (const heading of [
    'Submitting your work',
    'What we look for',
    'Your decision and report',
    'Publication and privacy',
    'Formalization and corrections',
  ]) {
    expect(screen.getByRole('heading', { name: heading })).toBeVisible();
  }
  expect(screen.getByText(/Uncheck this option if you want only a private review/)).toBeVisible();
  expect(screen.getByText(/interrupted review is not a mathematical rejection/)).toBeVisible();
  expect(screen.getByText(/AI review does not constitute a formal proof/)).toBeVisible();
  expect(document.body.textContent).not.toMatch(
    /S0|S1|S2|S3|Trureturing|bind-only|Phase B|request version|Filed automatically/,
  );
});
