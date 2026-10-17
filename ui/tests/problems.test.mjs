// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { createElement } from 'react';
import { cleanup, fireEvent, render, within } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { localize, message, previewLanguage, translate } from '../src/i18n';
import { rejectedDiagnostics } from '../src/workbench/batchEditing';
import { NoticeBanner } from '../src/workbench/Dialog';
import { ToolWindows } from '../src/workbench/ToolWindows';
import { initialState } from '../src/workbench/state';
import { withCommands } from './support/commands.mjs';
import { ProblemsTable, RejectionDetails } from '../src/workbench/Problems';

beforeEach(() => previewLanguage('en'));
afterEach(() => {
  cleanup();
  previewLanguage('system');
});

const object = (objectId, shortName, module) => ({
  objectId,
  parentId: null,
  kind: 'ECUC-CONTAINER-VALUE',
  shortName,
  definitionId: `/AUTOSAR/EcucDefs/${module}/${shortName}`,
  path: `/Ecuc/${shortName}`,
  sourceId: 'source',
  writable: true,
  reason: null,
});
const diagnostic = (code, severity, extra = {}) => ({
  scope: 'definition',
  severity,
  code,
  message: `${code} message`,
  remedy: null,
  file: null,
  path: null,
  sourceId: null,
  objectId: null,
  fieldId: null,
  ...extra,
});

function controller(extra = {}) {
  return {
    projection: {
      objects: [object('canif', 'CanIfInitCfg', 'CanIf'), object('com', 'ComConfig', 'Com')],
      fields: [
        {
          fieldId: 'period',
          objectId: 'com',
          definitionId: '/AUTOSAR/EcucDefs/Com/ComConfig/ComMainFunctionPeriod',
        },
      ],
      sources: [{ sourceId: 'source', path: 'Ecuc.arxml' }],
    },
    diagnostics: [
      diagnostic('COM_WARN', 'warning', { objectId: 'com' }),
      diagnostic('CANIF_ERROR', 'error', { objectId: 'canif' }),
      diagnostic('PERIOD_ERROR', 'error', { fieldId: 'period' }),
    ],
    issues: [],
    integrationIssues: [
      {
        category: 'input',
        code: 'STANDARD_INPUT',
        file: 'Ecuc.arxml',
        object: null,
        message: 'Standard message',
        remedy: 'Standard remedy',
      },
    ],
    rejection: null,
    activeObjectId: null,
    objectSelection: [],
    openDocument: vi.fn(),
    guardContext: vi.fn((_title, action) => action()),
    selectObject: vi.fn().mockResolvedValue(undefined),
    setActiveFieldId: vi.fn(),
    readSource: vi.fn(),
    setTreeVisible: vi.fn(),
    setInspectorVisible: vi.fn(),
    setInspectorTab: vi.fn(),
    setTreeFilter: vi.fn(),
    setObjectFilter: vi.fn(),
    ...extra,
  };
}

const codes = (view) =>
  Array.from(view.container.querySelectorAll('tbody tr'), (row) => row.dataset.problemCode);

test('problems from every check are one table, errors first per module, sortable by column', () => {
  const view = render(createElement(ProblemsTable, { controller: controller() }));
  expect(codes(view)).toEqual([
    'STANDARD_INPUT · Input',
    'CANIF_ERROR',
    'PERIOD_ERROR',
    'COM_WARN',
  ]);
  // A parameter problem names its object and parameter and belongs to the object's module.
  const period = view.container.querySelector('[data-problem-code="PERIOD_ERROR"]');
  expect(period.querySelectorAll('td')[1].textContent).toBe('Com');
  expect(period.querySelectorAll('td')[2].textContent).toBe(
    '/Ecuc/ComConfig · ComMainFunctionPeriod',
  );

  const moduleHeader = view.getByRole('columnheader', { name: 'Module' });
  expect(moduleHeader.getAttribute('aria-sort')).toBe('none');
  fireEvent.click(within(moduleHeader).getByRole('button'));
  expect(moduleHeader.getAttribute('aria-sort')).toBe('ascending');
  expect(codes(view)).toEqual([
    'STANDARD_INPUT · Input',
    'CANIF_ERROR',
    'PERIOD_ERROR',
    'COM_WARN',
  ]);
  fireEvent.click(within(moduleHeader).getByRole('button'));
  expect(moduleHeader.getAttribute('aria-sort')).toBe('descending');
  expect(codes(view)).toEqual([
    'PERIOD_ERROR',
    'COM_WARN',
    'CANIF_ERROR',
    'STANDARD_INPUT · Input',
  ]);
});

test('severity, module and text filters narrow the rows', () => {
  const view = render(createElement(ProblemsTable, { controller: controller() }));
  const severity = view.getByRole('button', { name: /Warning/ });
  expect(severity.textContent).toContain('1');
  fireEvent.click(severity);
  expect(severity.getAttribute('aria-pressed')).toBe('false');
  expect(codes(view)).not.toContain('COM_WARN');
  fireEvent.click(severity);

  fireEvent.change(view.getByRole('combobox', { name: 'Filter by module' }), {
    target: { value: 'Com' },
  });
  expect(codes(view)).toEqual(['PERIOD_ERROR', 'COM_WARN']);
  fireEvent.change(view.getByRole('textbox', { name: 'Filter problem text…' }), {
    target: { value: 'functionperiod' },
  });
  expect(codes(view)).toEqual(['PERIOD_ERROR']);
  fireEvent.change(view.getByRole('textbox', { name: 'Filter problem text…' }), {
    target: { value: 'nothing like this' },
  });
  expect(view.getByText('No problems match the filters.')).toBeTruthy();
});

test('a problem row is marked by click and located by double-click, Enter or its button', async () => {
  const c = controller();
  const view = render(createElement(ProblemsTable, { controller: c }));
  const period = view.container.querySelector('[data-problem-code="PERIOD_ERROR"]');
  fireEvent.click(period);
  expect(period.getAttribute('aria-current')).toBe('true');
  expect(c.selectObject).not.toHaveBeenCalled();
  fireEvent.doubleClick(period);
  await vi.waitFor(() => expect(c.setActiveFieldId).toHaveBeenCalledWith('period'));
  expect(c.selectObject).toHaveBeenCalledWith('com');
  expect(c.setTreeFilter).toHaveBeenCalledWith('');

  fireEvent.keyDown(view.container.querySelector('[data-problem-code="CANIF_ERROR"]'), {
    key: 'Enter',
  });
  await vi.waitFor(() => expect(c.selectObject).toHaveBeenCalledWith('canif'));

  // A standard-input problem without an object opens its file.
  const standard = view.container.querySelector('[data-problem-code="STANDARD_INPUT · Input"]');
  fireEvent.click(within(standard).getByRole('button', { name: 'View source' }));
  await vi.waitFor(() => expect(c.readSource).toHaveBeenCalledWith('source'));
});

// The shape `prepare_configuration_change` returns when a creation exceeds the upper bound.
const multiplicityRejection = [
  [
    message('backend.arxml.changes.configuration_diagnostic_context', {
      code: 'MULTIPLICITY',
      rule_id: 'DEF-MULTIPLICITY',
      scope: '"definition"',
      severity: '"error"',
      file: 'Ecuc.arxml',
      source_id: 'source',
      path: '/Ecuc/CanIfInitCfg',
      object_id: 'canif',
      field_id: '',
    }),
    'CanIfInitCfg has too many CanIfInitHohCfg entries (allowed 1..1, currently 2)',
    'Remove the extra entry',
  ],
  [
    message('backend.arxml.integration_diagnostic_context', {
      code: 'STANDARD_INPUT',
      category: '"input"',
      file: 'Other.arxml',
      object: '/Ecuc/Missing',
    }),
    'Standard message',
    'Standard remedy',
  ],
];

test('the diagnostics of a rejected change are read field by field from the backend reply', () => {
  const items = rejectedDiagnostics(multiplicityRejection);
  expect(items).toHaveLength(2);
  expect(items[0]).toMatchObject({
    severity: 'error',
    code: 'MULTIPLICITY',
    path: '/Ecuc/CanIfInitCfg',
    file: 'Ecuc.arxml',
    objectId: 'canif',
    fieldId: null,
    sourceId: 'source',
  });
  expect(localize(items[0].message)).toContain('allowed 1..1, currently 2');
  expect(localize(items[0].remedy)).toBe('Remove the extra entry');
  expect(items[1]).toMatchObject({ code: 'STANDARD_INPUT', path: '/Ecuc/Missing', objectId: null });
  // Severity arrives JSON-encoded; a warning stays a warning, anything unreadable an error.
  const withSeverity = (severity) =>
    rejectedDiagnostics([
      message('backend.arxml.changes.configuration_diagnostic_context', {
        ...multiplicityRejection[0][0].params,
        severity,
      }),
      'message',
      'remedy',
    ])[0].severity;
  expect(withSeverity('"warning"')).toBe('warning');
  expect(withSeverity('not json')).toBe('error');
});

test('rejection details split severity, code, message, path and file and locate the object', async () => {
  const c = controller({
    rejection: { items: rejectedDiagnostics(multiplicityRejection), detail: 'raw' },
  });
  const view = render(createElement(RejectionDetails, { controller: c }));
  const section = view.getByRole('region', { name: 'Last rejected change' });
  const [first, second] = section.querySelectorAll('.rejection-item');
  const field = (item, label) =>
    Array.from(item.querySelectorAll('dt')).find((term) => term.textContent === label)
      .nextElementSibling.textContent;
  expect(field(first, 'Severity')).toBe('Error');
  expect(field(first, 'Code')).toBe('MULTIPLICITY');
  expect(field(first, 'Message')).toContain('allowed 1..1, currently 2');
  expect(field(first, 'Object path')).toBe('/Ecuc/CanIfInitCfg');
  expect(field(first, 'Source file')).toBe('Ecuc.arxml');
  fireEvent.click(within(first).getByRole('button', { name: translate('action.locate') }));
  await vi.waitFor(() => expect(c.selectObject).toHaveBeenCalledWith('canif'));
  expect(c.guardContext).toHaveBeenCalled();
  // Neither the object nor the file of the second exists in the workspace.
  expect(within(second).queryByRole('button')).toBeNull();
  cleanup();

  // On the current object, locating keeps the rejected draft: no draft guard, no reselection.
  const onCurrent = controller({
    rejection: c.rejection,
    activeObjectId: 'canif',
    objectSelection: ['canif'],
  });
  const again = render(createElement(RejectionDetails, { controller: onCurrent }));
  fireEvent.click(again.getAllByRole('button', { name: translate('action.locate') })[0]);
  await vi.waitFor(() =>
    expect(onCurrent.openDocument).toHaveBeenCalledWith({ kind: 'configuration' }),
  );
  expect(onCurrent.guardContext).not.toHaveBeenCalled();
  expect(onCurrent.selectObject).not.toHaveBeenCalled();
});

test('a rejected change notice is one summary line with a details action', () => {
  const onDetails = vi.fn();
  const notice = { tone: 'error', text: 'too many entries', rejected: true };
  const view = render(createElement(NoticeBanner, { notice, onDetails, onDismiss: vi.fn() }));
  expect(view.getByRole('alert').textContent).toContain('too many entries');
  fireEvent.click(view.getByRole('button', { name: 'View details' }));
  expect(onDetails).toHaveBeenCalled();
  // Without a details target the full notice is kept.
  view.rerender(createElement(NoticeBanner, { notice }));
  expect(view.queryByRole('button', { name: 'View details' })).toBeNull();
});

test('automatic-check caption and summaries share type and spacing while keeping expandable coverage', () => {
  const style = document.createElement('style');
  style.textContent = readFileSync(path.join(import.meta.dirname, '../src/styles.css'), 'utf8');
  document.head.append(style);
  try {
    const c = withCommands({
      ...initialState(),
      ...controller(),
      toolWindow: 'problems',
      problemCount: 0,
    });
    c.projection.validation = ['source-safety', 'schema', 'definition', 'target-generation'].map(
      (scope) => ({
        scope,
        status: 'passed',
        coverage: [{ ruleId: 'witness', supported: true, reason: 'Coverage detail' }],
      }),
    );
    const view = render(createElement(ToolWindows, { controller: c }));
    const strip = view.container.querySelector('.validation-scopes');
    const caption = strip.querySelector(':scope > p');
    const summaries = [...strip.querySelectorAll('summary')];
    expect(summaries).toHaveLength(4);
    const metrics = (node) => {
      const css = getComputedStyle(node);
      return [
        css.fontSize,
        css.lineHeight,
        css.paddingTop,
        css.paddingBottom,
        css.marginTop,
        css.marginBottom,
      ];
    };
    for (const summary of summaries) {
      expect(metrics(summary)).toEqual(metrics(caption));
      expect(summary.parentElement.tagName).toBe('DETAILS');
      expect(getComputedStyle(summary.parentElement).marginTop).toBe('0px');
      expect(summary.parentElement.textContent).toContain('Coverage detail');
    }
  } finally {
    style.remove();
  }
});
