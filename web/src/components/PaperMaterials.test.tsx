import { act, render, screen, within } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { setLocale } from '../i18n/locale';
import { paperMaterials as materials } from '../test/fixtures';
import { PaperMaterials } from './PaperMaterials';

describe('paper materials', () => {
  it('renders available readings, slides, notes, visualizations and repository with translated copy', () => {
    render(<PaperMaterials materials={materials} />);
    const section = screen.getByRole('region', { name: 'Materials' });
    expect(within(section).getByRole('link', { name: 'English' })).toHaveAttribute(
      'href',
      materials.reading.en,
    );
    expect(within(section).getByRole('link', { name: 'Simplified Chinese' })).toHaveAttribute(
      'href',
      materials.reading.zh,
    );
    expect(within(section).getAllByRole('link', { name: 'PDF' })).toHaveLength(1);
    expect(within(section).getAllByRole('link', { name: 'Notes' })).toHaveLength(2);
    expect(
      within(section).getByRole('link', { name: 'Open interactive explanation' }),
    ).toHaveAttribute('href', materials.visualizations[0]!.url);
    expect(within(section).getByRole('link', { name: 'Repository' })).toHaveAttribute(
      'href',
      materials.repository_url,
    );
    expect(section).not.toHaveTextContent('not verification');
    expect(screen.getByTitle('Plain-language reading')).toHaveAttribute(
      'src',
      materials.reading.en,
    );
    act(() => setLocale('zh'));
    expect(screen.getByRole('heading', { name: '论文资料' })).toBeInTheDocument();
    expect(screen.queryByText('由 Mathesis 根据原稿编写；解读与图示不代表验证。')).toBeNull();
    expect(screen.getByTitle('通俗解读')).toHaveAttribute('src', materials.reading.zh);
    expect(screen.getByRole('heading', { name: '研讨会幻灯片' })).toBeInTheDocument();
  });

  it('omits absent materials and missing media groups, retaining a published repository', () => {
    const { container, rerender } = render(<PaperMaterials />);
    expect(container).toBeEmptyDOMElement();
    rerender(
      <PaperMaterials materials={{ ...materials, reading: {}, slides: [], visualizations: [] }} />,
    );
    expect(screen.getAllByRole('link')).toHaveLength(2);
    expect(screen.queryByRole('heading', { name: 'Seminar slides' })).not.toBeInTheDocument();
    expect(
      screen.queryByRole('heading', { name: 'Interactive visualizations' }),
    ).not.toBeInTheDocument();
  });

  it('shows playable video and named shared requests without claiming verification', () => {
    render(
      <PaperMaterials
        materials={{
          ...materials,
          videos: [
            {
              id: 'intro-wide',
              title: 'Why parity matters',
              url: materials.site_url + 'films/',
              mp4_url: materials.site_url + 'intro.mp4',
            },
          ],
          formalization_requests: [
            {
              label: 'Theorem 1',
              url: 'https://github.com/the-omega-institute/trureturing-requests/issues/2',
            },
          ],
        }}
      />,
    );
    expect(screen.getByRole('link', { name: 'Why parity matters' })).toHaveAttribute(
      'href',
      materials.site_url + 'films/',
    );
    const video = screen.getByLabelText('Why parity matters');
    expect(video).toHaveAttribute('controls');
    expect(video.querySelector('source')).toHaveAttribute('src', materials.site_url + 'intro.mp4');
    expect(screen.getByRole('link', { name: 'Theorem 1' })).toHaveAttribute(
      'href',
      'https://github.com/the-omega-institute/trureturing-requests/issues/2',
    );
    expect(screen.queryByText('Lean ✓')).not.toBeInTheDocument();
  });
});
