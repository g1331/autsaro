// @vitest-environment jsdom
import { createElement, Fragment } from 'react';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { cleanup, fireEvent, render } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';
import { Tabs, tabPanelProps } from '../src/workbench/Tabs';
import { objectRole } from '../src/workbench/objectRole';
import { resources } from '../src/i18n';

afterEach(cleanup);

const items = ['one', 'two', 'three'].map((key) => ({ key, label: key.toUpperCase() }));
function renderTabs(props) {
  const onSelect = vi.fn();
  const view = render(
    createElement(
      Fragment,
      null,
      createElement(Tabs, { id: 't', label: 'Views', items, selected: 'two', onSelect, ...props }),
      createElement('div', tabPanelProps('t', 'two'), 'panel'),
    ),
  );
  return { ...view, onSelect, tabs: view.getAllByRole('tab') };
}

test('tabs expose one tab stop and connect tabs to the panel', () => {
  const { tabs, getByRole } = renderTabs();
  expect(tabs.map((tab) => tab.tabIndex)).toEqual([-1, 0, -1]);
  expect(tabs[1].getAttribute('aria-selected')).toBe('true');
  const panel = getByRole('tabpanel');
  expect(tabs.every((tab) => tab.getAttribute('aria-controls') === panel.id)).toBe(true);
  expect(panel.getAttribute('aria-labelledby')).toBe(tabs[1].id);
});

test('arrow, Home and End keys move focus and selection, wrapping at the ends', () => {
  const { tabs, onSelect } = renderTabs();
  tabs[1].focus();
  fireEvent.keyDown(tabs[1], { key: 'ArrowRight' });
  expect(document.activeElement).toBe(tabs[2]);
  fireEvent.keyDown(tabs[2], { key: 'ArrowRight' });
  expect(document.activeElement).toBe(tabs[0]);
  fireEvent.keyDown(tabs[0], { key: 'ArrowLeft' });
  expect(document.activeElement).toBe(tabs[2]);
  fireEvent.keyDown(tabs[2], { key: 'Home' });
  expect(document.activeElement).toBe(tabs[0]);
  fireEvent.keyDown(tabs[0], { key: 'End' });
  expect(document.activeElement).toBe(tabs[2]);
  expect(onSelect.mock.calls.map(([key]) => key)).toEqual([
    'three',
    'one',
    'three',
    'one',
    'three',
  ]);
  fireEvent.keyDown(tabs[2], { key: 'ArrowDown' });
  expect(document.activeElement).toBe(tabs[2]);
});

test('vertical tabs use up and down arrows', () => {
  const { tabs, onSelect } = renderTabs({ orientation: 'vertical' });
  fireEvent.keyDown(tabs[1], { key: 'ArrowDown' });
  expect(document.activeElement).toBe(tabs[2]);
  fireEvent.keyDown(tabs[2], { key: 'ArrowRight' });
  expect(document.activeElement).toBe(tabs[2]);
  expect(onSelect).toHaveBeenCalledTimes(1);
});

test('manual tabs move focus without activating, and Delete closes a closable tab', () => {
  const onClose = vi.fn();
  const { tabs, onSelect, getAllByRole } = renderTabs({
    manual: true,
    items: items.map((item) => ({
      ...item,
      onClose: () => onClose(item.key),
      closeLabel: 'Close ' + item.key,
    })),
  });
  fireEvent.keyDown(tabs[1], { key: 'ArrowRight' });
  expect(document.activeElement).toBe(tabs[2]);
  expect(onSelect).not.toHaveBeenCalled();
  fireEvent.click(tabs[2]);
  expect(onSelect).toHaveBeenCalledWith('three');
  fireEvent.keyDown(tabs[2], { key: 'Delete' });
  expect(onClose).toHaveBeenCalledWith('three');
  const close = getAllByRole('button', { name: 'Close one' })[0];
  expect(close.tabIndex).toBe(-1);
  expect(close.closest('[role="tab"]')).toBeNull();
});

test('object roles follow the ECUC module or the element kind', () => {
  const ecuc = (module, kind = 'ECUC-CONTAINER-VALUE') => ({
    kind,
    definitionId: `/AUTOSAR/EcucDefs/${module}/X`,
  });
  expect(objectRole(ecuc('CanIf', 'ECUC-MODULE-CONFIGURATION-VALUES'))).toBe('blue');
  expect(objectRole(ecuc('PduR'))).toBe('blue');
  expect(objectRole(ecuc('Dcm'))).toBe('gold');
  expect(objectRole(ecuc('Dem'))).toBe('gold');
  expect(objectRole(ecuc('Os'))).toBe('teal');
  expect(objectRole({ kind: 'ECUC-MODULE-CONFIGURATION-VALUES', definitionId: null })).toBe('teal');
  expect(objectRole({ kind: 'APPLICATION-SW-COMPONENT-TYPE', definitionId: null })).toBe('purple');
  expect(objectRole({ kind: 'IMPLEMENTATION-DATA-TYPE', definitionId: null })).toBe('purple');
  expect(objectRole({ kind: 'I-SIGNAL-I-PDU', definitionId: null })).toBe('blue');
  expect(objectRole({ kind: 'DIAGNOSTIC-DATA-IDENTIFIER', definitionId: null })).toBe('gold');
  expect(objectRole({ kind: 'AR-PACKAGE', definitionId: null })).toBeNull();
});

// DESIGN.md is the visual authority; the stylesheet must carry the same colour tokens.
const design = readFileSync(path.join(import.meta.dirname, '../../DESIGN.md'), 'utf8');
const css = readFileSync(path.join(import.meta.dirname, '../src/styles.css'), 'utf8');
const designColors = Object.fromEntries(
  [...design.split('\n---\n')[0].matchAll(/^ {2}([a-z-]+): '(#[0-9a-f]{6})'$/gm)].map((m) => [
    m[1],
    m[2],
  ]),
);
const cssAlias = { 'text-primary': 'text', 'text-secondary': 'secondary', 'text-muted': 'muted' };
function cssTokens(selector) {
  const start = css.indexOf(`${selector} {`);
  const block = css.slice(start, css.indexOf('\n}', start));
  return Object.fromEntries(
    [...block.matchAll(/--([a-z-]+): (#[0-9a-f]{6});/g)].map((m) => [m[1], m[2]]),
  );
}
const themes = { light: cssTokens(':root'), dark: cssTokens(":root[data-theme='dark']") };

test('every DESIGN.md colour token has the same value in the stylesheet', () => {
  const names = Object.keys(designColors).filter((name) => !name.startsWith('dark-'));
  expect(names.length).toBeGreaterThan(20);
  for (const name of names) {
    const property = cssAlias[name] ?? name;
    expect([name, themes.light[property]]).toEqual([name, designColors[name]]);
    expect([name, themes.dark[property]]).toEqual([name, designColors[`dark-${name}`]]);
  }
});

function luminance(hex) {
  const channel = (offset) => {
    const value = parseInt(hex.slice(offset, offset + 2), 16) / 255;
    return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5);
}
const contrast = (a, b) => {
  const [high, low] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (high + 0.05) / (low + 0.05);
};

test('control boundaries reach 3:1 against every background in both themes (WCAG 1.4.11)', () => {
  for (const [theme, tokens] of Object.entries(themes))
    for (const background of ['shell', 'surface', 'canvas', 'overlay'])
      expect([
        theme,
        background,
        contrast(tokens['border-strong'], tokens[background]) >= 3,
      ]).toEqual([theme, background, true]);
});

test('Chinese copy has no ASCII space after full-width punctuation', () => {
  const spaced = Object.entries(resources['zh-CN'].translation).filter(([, value]) =>
    /[，。；：！？、] /.test(String(value)),
  );
  expect(spaced).toEqual([]);
});
