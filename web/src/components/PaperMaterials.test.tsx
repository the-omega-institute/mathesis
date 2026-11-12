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
    expect(within(section).getByRole('link', { name: 'parity-demo' })).toHaveAttribute(
      'href',
      materials.visualizations[0]!.url,
    );
    expect(within(section).getByRole('link', { name: 'Repository' })).toHaveAttribute(
      'href',
      materials.repository_url,
    );
    expect(section).toHaveTextContent(
      'Prepared by Mathesis from the manuscript; explanations and illustrations are not verification.',
    );
    act(() => setLocale('zh'));
    expect(screen.getByRole('heading', { name: '论文资料' })).toBeInTheDocument();
    expect(
      screen.getByText('由 Mathesis 根据原稿编写；解读与图示不代表验证。'),
    ).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: '研讨会幻灯片' })).toBeInTheDocument();
  });

  it('omits absent materials and missing media groups, retaining a published repository', () => {
    const { container, rerender } = render(<PaperMaterials />);
    expect(container).toBeEmptyDOMElement();
    rerender(
      <PaperMaterials materials={{ ...materials, reading: {}, slides: [], visualizations: [] }} />,
    );
    expect(screen.getAllByRole('link')).toHaveLength(1);
    expect(screen.queryByRole('heading', { name: 'Seminar slides' })).not.toBeInTheDocument();
    expect(
      screen.queryByRole('heading', { name: 'Interactive visualizations' }),
    ).not.toBeInTheDocument();
  });
});
