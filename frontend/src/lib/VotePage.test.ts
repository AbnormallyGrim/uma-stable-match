// VotePage component tests. The polls are StrawPoll's; only their frames are checked.

import { render, screen, within } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import { POLLS } from './polls';
import VotePage from './VotePage.svelte';

describe('VotePage', () => {
  it('has a heading and sets the page title', () => {
    render(VotePage);
    expect(
      screen.getByRole('heading', { level: 1, name: 'Help decide what gets built next' }),
    ).toBeInTheDocument();
    expect(document.title).toBe('Vote on ideas · Uma Stable Match');
  });

  it('embeds each poll in a frame from StrawPoll', () => {
    const { container } = render(VotePage);
    const frames = container.querySelectorAll('iframe');
    expect(frames).toHaveLength(POLLS.length);
    frames.forEach((frame, i) => {
      expect(frame).toHaveAttribute('src', `https://strawpoll.com/embed/${POLLS[i]!.id}`);
      expect(frame).toHaveAccessibleName();
      expect(frame.style.height).toBe(`${POLLS[i]!.height}px`);
    });
  });

  it('says StrawPoll hosts the polls, with a link to its privacy policy', () => {
    render(VotePage);
    const about = screen.getByRole('complementary', { name: 'About these polls' });
    expect(about).toHaveTextContent('hosted by StrawPoll, not by Uma Stable Match');
    const policy = within(about).getByRole('link', { name: 'its privacy policy' });
    expect(policy).toHaveAttribute('href', 'https://strawpoll.com/privacy');
    expect(policy).toHaveAttribute('rel', 'noopener noreferrer');
  });

  it('has the navbar and footer', () => {
    render(VotePage);
    expect(screen.getByRole('navigation', { name: 'Main' })).toBeInTheDocument();
    expect(screen.getByRole('contentinfo')).toBeInTheDocument();
  });
});
