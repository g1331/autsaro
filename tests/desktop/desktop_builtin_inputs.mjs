import { mkdir, writeFile, readFile, readdir } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import path from 'node:path';

const root = '/AUTOSAR/EcucDefs/';
const escape = (text) => text.replaceAll('&', '&amp;').replaceAll('<', '&lt;');
const document = (elements, name = 'Acceptance') =>
  `<?xml version="1.0" encoding="UTF-8"?>\n<AUTOSAR xmlns="http://autosar.org/schema/r4.0" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:schemaLocation="http://autosar.org/schema/r4.0 AUTOSAR_00053.xsd"><AR-PACKAGES><AR-PACKAGE><SHORT-NAME>${name}</SHORT-NAME><ELEMENTS>${elements}</ELEMENTS></AR-PACKAGE></AR-PACKAGES></AUTOSAR>\n`;
const module = (name, children) =>
  `<ECUC-MODULE-CONFIGURATION-VALUES><SHORT-NAME>${name}</SHORT-NAME><DEFINITION-REF DEST="ECUC-MODULE-DEF">${root}${name}</DEFINITION-REF><CONTAINERS>${children}</CONTAINERS></ECUC-MODULE-CONFIGURATION-VALUES>`;
const container = (definition, name, values = '', children = '', refs = '') =>
  `<ECUC-CONTAINER-VALUE><SHORT-NAME>${name}</SHORT-NAME><DEFINITION-REF DEST="ECUC-PARAM-CONF-CONTAINER-DEF">${root}${definition}</DEFINITION-REF>${values ? `<PARAMETER-VALUES>${values}</PARAMETER-VALUES>` : ''}${refs ? `<REFERENCE-VALUES>${refs}</REFERENCE-VALUES>` : ''}${children ? `<SUB-CONTAINERS>${children}</SUB-CONTAINERS>` : ''}</ECUC-CONTAINER-VALUE>`;
const field = (definition, kind, lexeme) => {
  const tag = ['INTEGER', 'FLOAT', 'BOOLEAN'].includes(kind)
    ? 'ECUC-NUMERICAL-PARAM-VALUE'
    : 'ECUC-TEXTUAL-PARAM-VALUE';
  const dest = kind === 'FUNCTION-NAME' ? 'ECUC-FUNCTION-NAME-DEF' : `ECUC-${kind}-PARAM-DEF`;
  return `<${tag}><DEFINITION-REF DEST="${dest}">${root}${definition}</DEFINITION-REF><VALUE>${escape(lexeme)}</VALUE></${tag}>`;
};
const reference = (definition, target) =>
  `<ECUC-REFERENCE-VALUE><DEFINITION-REF DEST="ECUC-REFERENCE-DEF">${root}${definition}</DEFINITION-REF><VALUE-REF DEST="ECUC-CONTAINER-VALUE">${target}</VALUE-REF></ECUC-REFERENCE-VALUE>`;

export async function createBuiltinInputs(scratch) {
  const directory = path.join(scratch, 'authored-inputs');
  await mkdir(directory);
  const task = 'Os/OsTask';
  const os = module(
    'Os',
    container(
      'Os/OsEvent',
      'DataReady',
      field('Os/OsEvent/OsEventMask', 'INTEGER', '9007199254740993'),
    ) +
      container('Os/OsEvent', 'DiagnosticReady', field('Os/OsEvent/OsEventMask', 'INTEGER', '16')) +
      container(
        task,
        'Receiver',
        [
          field(`${task}/OsTaskPriority`, 'INTEGER', '3'),
          field(`${task}/OsTaskActivation`, 'INTEGER', '1'),
          field(`${task}/OsTaskSchedule`, 'ENUMERATION', 'FULL'),
        ].join(''),
        '',
        reference(`${task}/OsTaskEventRef`, '/Acceptance/Os/DataReady'),
      ),
  );
  const com = module(
    'Com',
    container(
      'Com/ComConfig',
      'ComConfig',
      '',
      container(
        'Com/ComConfig/ComSignal',
        'Sample',
        [
          field('Com/ComConfig/ComSignal/ComBitPosition', 'INTEGER', '0'),
          field('Com/ComConfig/ComSignal/ComBitSize', 'INTEGER', '32'),
          field('Com/ComConfig/ComSignal/ComSignalType', 'ENUMERATION', 'UINT32'),
          field('Com/ComConfig/ComSignal/ComSignalEndianness', 'ENUMERATION', 'LITTLE_ENDIAN'),
          field('Com/ComConfig/ComSignal/ComTimeout', 'FLOAT', '0.0250'),
          field('Com/ComConfig/ComSignal/ComSignalInitValue', 'STRING', '27'),
        ].join(''),
      ),
    ),
  );
  const can = module(
    'Can',
    container(
      'Can/CanGeneral',
      'General',
      field('Can/CanGeneral/CanDevErrorDetect', 'BOOLEAN', 'false'),
    ),
  );
  const canIf = module(
    'CanIf',
    container(
      'CanIf/CanIfInitCfg',
      'Init',
      field('CanIf/CanIfInitCfg/CanIfInitCfgSet', 'STRING', 'AcceptanceInit'),
    ),
  );
  const data = 'Dcm/DcmConfigSet/DcmDsp/DcmDspData';
  const dcm = module(
    'Dcm',
    container(
      'Dcm/DcmConfigSet',
      'Config',
      '',
      container(
        'Dcm/DcmConfigSet/DcmDsp',
        'Dsp',
        '',
        container(
          data,
          'SampleReader',
          [
            field(`${data}/DcmDspDataType`, 'ENUMERATION', 'UINT8_N'),
            field(`${data}/DcmDspDataUsePort`, 'ENUMERATION', 'USE_DATA_SYNCH_FNC'),
            field(`${data}/DcmDspDataReadFnc`, 'FUNCTION-NAME', 'Acceptance_Read'),
          ].join(''),
        ),
      ),
    ),
  );
  // Missing required siblings intentionally create pre-existing definition
  // witnesses. Repairing independent fields must not turn target failure into a
  // global source-save gate or silently synthesize these missing values.
  // Valid R24-11 syntax outside the bounded editable standard vocabulary.
  const unknown = '<SW-ADDR-METHOD><SHORT-NAME>PreservedType</SHORT-NAME></SW-ADDR-METHOD>';
  const members = {
    'os.arxml': os,
    'com.arxml': com,
    'can.arxml': can + canIf,
    'dcm.arxml': dcm,
    'unknown.arxml': unknown,
  };
  const sources = [];
  for (const [name, elements] of Object.entries(members)) {
    const source = path.join(directory, name);
    await writeFile(source, document(elements));
    sources.push(source);
  }
  const safety = {};
  for (const [name, contents] of Object.entries({
    malformed: '<AUTOSAR><broken></AUTOSAR>',
    dtd: '<!DOCTYPE AUTOSAR [<!ENTITY value "forbidden">]><AUTOSAR/>',
    entity: '<!DOCTYPE AUTOSAR SYSTEM "https://invalid.example/autosar.dtd"><AUTOSAR/>',
    encoding: Buffer.from([0xff, 0xfe, 0x41, 0x00]),
  })) {
    safety[name] = path.join(directory, `${name}.arxml`);
    await writeFile(safety[name], contents);
  }
  safety.oversized = path.join(directory, 'oversized.arxml');
  await writeFile(safety.oversized, Buffer.alloc(50 * 1024 * 1024 + 1, 0x20));
  const large = path.join(directory, 'large.arxml');
  const expanded = Array.from(
    { length: 16000 },
    (_, index) =>
      `<IMPLEMENTATION-DATA-TYPE><SHORT-NAME>Preserved_${index}</SHORT-NAME><CATEGORY>VALUE</CATEGORY><ADMIN-DATA><LANGUAGE>EN</LANGUAGE></ADMIN-DATA></IMPLEMENTATION-DATA-TYPE>`,
  ).join('');
  await writeFile(large, document(expanded));
  return { sources, safety, large, unknown };
}

export async function createExtensionInputs(scratch) {
  const base = path.join(scratch, 'authored-extension');
  await mkdir(base);
  const parameters = [
    '<ECUC-INTEGER-PARAM-DEF><SHORT-NAME>Counter</SHORT-NAME><LOWER-MULTIPLICITY>0</LOWER-MULTIPLICITY><UPPER-MULTIPLICITY>1</UPPER-MULTIPLICITY><MIN>0</MIN><MAX>18446744073709551615</MAX><DEFAULT-VALUE>9007199254740993</DEFAULT-VALUE></ECUC-INTEGER-PARAM-DEF>',
  ].join('');
  const xml = document(
    `<ECUC-MODULE-DEF><SHORT-NAME>Lab</SHORT-NAME><LOWER-MULTIPLICITY>0</LOWER-MULTIPLICITY><UPPER-MULTIPLICITY>1</UPPER-MULTIPLICITY><CONTAINERS><ECUC-PARAM-CONF-CONTAINER-DEF><SHORT-NAME>Config</SHORT-NAME><LOWER-MULTIPLICITY>0</LOWER-MULTIPLICITY><UPPER-MULTIPLICITY>1</UPPER-MULTIPLICITY><PARAMETERS>${parameters}</PARAMETERS></ECUC-PARAM-CONF-CONTAINER-DEF></CONTAINERS></ECUC-MODULE-DEF>`,
    'AcceptanceVendor',
  );
  await writeFile(path.join(base, 'lab.arxml'), xml);
  const inventory = {
    formatVersion: 1,
    catalogId: 'acceptance-lab',
    release: 'R24-11',
    version: '1.0.0',
    files: [
      {
        path: 'lab.arxml',
        sha256: createHash('sha256').update(xml).digest('hex'),
      },
    ],
  };
  const valid = path.join(base, 'catalog.json');
  await writeFile(valid, JSON.stringify(inventory));
  const variants = {};
  for (const [name, modified] of Object.entries({
    release: { ...inventory, release: 'R23-11' },
    digest: {
      ...inventory,
      files: [{ ...inventory.files[0], sha256: '0'.repeat(64) }],
    },
    escape: {
      ...inventory,
      files: [{ ...inventory.files[0], path: '../lab.arxml' }],
    },
  })) {
    const directory = path.join(scratch, `extension-reject-${name}`);
    await mkdir(directory);
    await writeFile(path.join(directory, 'lab.arxml'), xml);
    variants[name] = path.join(directory, 'catalog.json');
    await writeFile(variants[name], JSON.stringify(modified));
  }
  const conflictDirectory = path.join(scratch, 'extension-reject-builtin-conflict');
  await mkdir(conflictDirectory);
  const conflictXml =
    '<?xml version="1.0"?><AUTOSAR xmlns="http://autosar.org/schema/r4.0"><AR-PACKAGES><AR-PACKAGE><SHORT-NAME>AUTOSAR</SHORT-NAME><AR-PACKAGES><AR-PACKAGE><SHORT-NAME>EcucDefs</SHORT-NAME><ELEMENTS><ECUC-MODULE-DEF><SHORT-NAME>Can</SHORT-NAME><LOWER-MULTIPLICITY>0</LOWER-MULTIPLICITY><UPPER-MULTIPLICITY>1</UPPER-MULTIPLICITY></ECUC-MODULE-DEF></ELEMENTS></AR-PACKAGE></AR-PACKAGES></AR-PACKAGE></AR-PACKAGES></AUTOSAR>';
  await writeFile(path.join(conflictDirectory, 'lab.arxml'), conflictXml);
  variants.conflict = path.join(conflictDirectory, 'catalog.json');
  await writeFile(
    variants.conflict,
    JSON.stringify({
      ...inventory,
      catalogId: 'forbidden-builtin-override',
      files: [
        {
          path: 'lab.arxml',
          sha256: createHash('sha256').update(conflictXml).digest('hex'),
        },
      ],
    }),
  );
  return {
    valid,
    variants,
    identity: {
      catalogId: inventory.catalogId,
      release: inventory.release,
      version: inventory.version,
      sha256: createHash('sha256').update(JSON.stringify(inventory)).digest('hex'),
    },
    payload: path.join(base, 'lab.arxml'),
  };
}

// Reuse the project's original source fixture, outside the application's tool boundary.
export async function createMultiComponentInputs(scratch) {
  const fixture = new URL('../../core/tests/fixtures/multi-component/', import.meta.url);
  const directory = path.join(scratch, 'projects', 'MultiComponent');
  await mkdir(directory);
  const names = (await readdir(fixture)).filter((name) => name.endsWith('.arxml')).sort();
  for (const name of names)
    await writeFile(path.join(directory, name), await readFile(new URL(name, fixture)));
  const manifest = path.join(directory, 'workbench-project.json');
  await writeFile(
    manifest,
    JSON.stringify(
      {
        formatVersion: 1,
        declaredRelease: 'R24-11',
        profileHint: 'singlecore-multi-swc-v1',
        inputs: names.map((name) => ({ path: name, roleHint: 'standard' })),
        applicationInputs: [],
        acceptedExtensionDefinitions: [],
      },
      null,
      2,
    ),
  );
  return { directory, manifest };
}
