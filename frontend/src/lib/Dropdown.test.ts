// Dropdown component tests.

import { fireEvent, render, screen } from '@testing-library/svelte';
import { describe, expect, it } from 'vitest';

import Dropdown from './Dropdown.svelte';

const options = [
  { value: null, label: 'Any playstyle' },
  { value: 2, label: 'Laid-back' },
  { value: 3, label: 'Going for Gold' },
  { value: 12, label: 'Log in Daily' },
];

function setup(value: number | null = null) {
  render(Dropdown, { id: 'pick', labelledby: 'pick-label', options, value });
  return screen.getByRole('combobox');
}

describe('Dropdown', () => {
  it('shows the selected option and starts closed', () => {
    const trigger = setup(3);
    expect(trigger).toHaveTextContent('Going for Gold');
    expect(trigger).toHaveAttribute('aria-expanded', 'false');
    expect(screen.queryByRole('listbox')).not.toBeInTheDocument();
  });

  it('opens on click and marks the selected option', async () => {
    const trigger = setup(2);
    await fireEvent.click(trigger);
    expect(trigger).toHaveAttribute('aria-expanded', 'true');
    expect(screen.getByRole('option', { name: 'Laid-back' })).toHaveAttribute(
      'aria-selected',
      'true',
    );
    expect(trigger).toHaveAttribute('aria-activedescendant', 'pick-option-1');
  });

  it('picks an option with the arrow keys and Enter', async () => {
    const trigger = setup();
    await fireEvent.keyDown(trigger, { key: 'ArrowDown' });
    expect(screen.getByRole('listbox')).toBeInTheDocument();
    await fireEvent.keyDown(trigger, { key: 'ArrowDown' });
    await fireEvent.keyDown(trigger, { key: 'ArrowDown' });
    expect(trigger).toHaveAttribute('aria-activedescendant', 'pick-option-2');
    await fireEvent.keyDown(trigger, { key: 'Enter' });
    expect(trigger).toHaveTextContent('Going for Gold');
    expect(screen.queryByRole('listbox')).not.toBeInTheDocument();
  });

  it('jumps to an option by typing and supports Home and End', async () => {
    const trigger = setup();
    await fireEvent.click(trigger);
    await fireEvent.keyDown(trigger, { key: 'l' });
    expect(trigger).toHaveAttribute('aria-activedescendant', 'pick-option-1');
    await fireEvent.keyDown(trigger, { key: 'l' });
    expect(trigger).toHaveAttribute('aria-activedescendant', 'pick-option-3');
    await fireEvent.keyDown(trigger, { key: 'Home' });
    expect(trigger).toHaveAttribute('aria-activedescendant', 'pick-option-0');
    await fireEvent.keyDown(trigger, { key: 'End' });
    expect(trigger).toHaveAttribute('aria-activedescendant', 'pick-option-3');
  });

  it('closes on Escape without changing the value', async () => {
    const trigger = setup(2);
    await fireEvent.click(trigger);
    await fireEvent.keyDown(trigger, { key: 'ArrowDown' });
    await fireEvent.keyDown(trigger, { key: 'Escape' });
    expect(screen.queryByRole('listbox')).not.toBeInTheDocument();
    expect(trigger).toHaveTextContent('Laid-back');
  });

  it('closes when clicking outside', async () => {
    const trigger = setup();
    await fireEvent.click(trigger);
    await fireEvent.pointerDown(document.body);
    expect(screen.queryByRole('listbox')).not.toBeInTheDocument();
  });
});
