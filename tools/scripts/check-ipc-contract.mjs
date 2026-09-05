import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

export function extractRegisteredCommands(source) {
  const handler = source.match(/tauri::generate_handler!\[([\s\S]*?)\]\)/)?.[1];
  if (!handler) throw new Error('Cannot find tauri::generate_handler! command registry');
  return handler
    .split(',')
    .map((entry) => entry.trim().split('::').at(-1))
    .filter(Boolean)
    .sort();
}

export function extractAnnotatedCommands(sources) {
  return sources
    .flatMap((source) =>
      [
        ...source.matchAll(
          /#\[(?:tauri::)?command(?:\([^\]]*\))?\]\s*pub\s+async\s+fn\s+([a-zA-Z0-9_]+)/g,
        ),
      ].map((match) => match[1]),
    )
    .sort();
}

export function extractRustCommandSignatures(sources) {
  const signatures = {};
  for (const source of sources) {
    const commandPattern = /#\[(?:tauri::)?command(?:\([^\]]*\))?\]\s*pub\s+async\s+fn\s+([a-zA-Z0-9_]+)\s*\(/g;
    for (const match of source.matchAll(commandPattern)) {
      const openParenthesis = match.index + match[0].length - 1;
      const closeParenthesis = findMatching(source, openParenthesis, '(', ')');
      const parameters = splitTopLevel(source.slice(openParenthesis + 1, closeParenthesis), ',')
        .map((parameter) => parameter.trim())
        .filter(Boolean)
        .map(parseRustParameter)
        .filter(Boolean);
      const returnMatch = source.slice(closeParenthesis + 1).match(/^\s*->\s*Result\s*</);
      if (!returnMatch) throw new Error(`Cannot parse Rust result for ${match[1]}`);
      const resultOpen = closeParenthesis + 1 + returnMatch.index + returnMatch[0].length - 1;
      const resultClose = findMatching(source, resultOpen, '<', '>');
      const [successType] = splitTopLevel(source.slice(resultOpen + 1, resultClose), ',');
      signatures[match[1]] = {
        args: parameters,
        result: normalizeRustType(successType.trim()),
      };
    }
  }
  return signatures;
}

export function extractTypeScriptCommandSignatures(source) {
  const body = extractInterfaceBody(source, 'CommandMap');
  const signatures = {};
  const entryPattern = /^ {2}(?:'([^']+)'|([a-zA-Z0-9_]+))\s*:\s*\{/gm;
  for (const match of body.matchAll(entryPattern)) {
    const name = match[1] ?? match[2];
    const openBrace = match.index + match[0].length - 1;
    const closeBrace = findMatching(body, openBrace, '{', '}');
    const commandBody = body.slice(openBrace + 1, closeBrace);
    const argsType = extractPropertyType(commandBody, 'args');
    const resultType = extractPropertyType(commandBody, 'result');
    signatures[name] = {
      args: parseTypeScriptArguments(argsType),
      result: normalizeTypeScriptType(resultType),
    };
  }
  return signatures;
}

export function extractInterfaceKeys(source, interfaceName) {
  const body = extractInterfaceBody(source, interfaceName);
  return [...body.matchAll(/^ {2}(?:'([^']+)'|([a-zA-Z0-9_]+))\s*:/gm)]
    .map((match) => match[1] ?? match[2])
    .sort();
}

export function assertCommandSignatures(rustSignatures, typescriptSignatures) {
  for (const name of Object.keys(rustSignatures).sort()) {
    const rust = rustSignatures[name];
    const typescript = typescriptSignatures[name];
    if (!typescript) continue;
    if (JSON.stringify(rust) !== JSON.stringify(typescript)) {
      throw new Error(
        `IPC signature drift for ${name}: Rust ${JSON.stringify(rust)}, TypeScript ${JSON.stringify(typescript)}`,
      );
    }
  }
}

export function extractRustEvents(source) {
  return [...source.matchAll(/pub const EVENT_[A-Z0-9_]+: &str = "([^"]+)";/g)]
    .map((match) => match[1])
    .sort();
}

export function assertSameContract(label, rustEntries, typescriptEntries) {
  const rustOnly = rustEntries.filter((entry) => !typescriptEntries.includes(entry));
  const typescriptOnly = typescriptEntries.filter((entry) => !rustEntries.includes(entry));
  if (rustOnly.length > 0 || typescriptOnly.length > 0) {
    throw new Error(
      `${label} drift: Rust-only [${rustOnly.join(', ')}], TypeScript-only [${typescriptOnly.join(', ')}]`,
    );
  }
}

export function verifyIpcContract(rootDir) {
  const rustEntry = fs.readFileSync(path.join(rootDir, 'src-tauri/src/lib.rs'), 'utf8');
  const commandDirectory = path.join(rootDir, 'src-tauri/src/commands');
  const commandSources = fs
    .readdirSync(commandDirectory)
    .filter((file) => file.endsWith('.rs'))
    .map((file) => fs.readFileSync(path.join(commandDirectory, file), 'utf8'));
  const contractSource = fs.readFileSync(
    path.join(rootDir, 'apps/desktop/src/shared/api/contracts/commandMap.ts'),
    'utf8',
  );
  const registered = extractRegisteredCommands(rustEntry);
  const annotated = extractAnnotatedCommands(commandSources);
  const typed = extractInterfaceKeys(contractSource, 'CommandMap');
  assertSameContract('registered versus annotated commands', registered, annotated);
  assertSameContract('registered commands versus CommandMap', registered, typed);
  assertCommandSignatures(
    extractRustCommandSignatures(commandSources),
    extractTypeScriptCommandSignatures(contractSource),
  );

  const eventPublisher = fs.readFileSync(
    path.join(rootDir, 'crates/adapters-tauri/src/event_publisher.rs'),
    'utf8',
  );
  const rustEvents = extractRustEvents(eventPublisher);
  const typedEvents = extractInterfaceKeys(contractSource, 'EventMap');
  assertSameContract('Rust events versus EventMap', rustEvents, typedEvents);
  if (/\.emit\(\s*"/.test([...commandSources, eventPublisher].join('\n'))) {
    throw new Error('Tauri events must use the centralized EVENT_* registry');
  }
}

function extractInterfaceBody(source, interfaceName) {
  const marker = `export interface ${interfaceName}`;
  const markerIndex = source.indexOf(marker);
  if (markerIndex < 0) throw new Error(`Cannot find TypeScript ${interfaceName}`);
  const openIndex = source.indexOf('{', markerIndex);
  const closeIndex = findMatching(source, openIndex, '{', '}');
  return source.slice(openIndex + 1, closeIndex);
}

function findMatching(source, openIndex, openCharacter, closeCharacter) {
  let depth = 0;
  for (let index = openIndex; index < source.length; index += 1) {
    if (source[index] === openCharacter) depth += 1;
    if (source[index] === closeCharacter) depth -= 1;
    if (depth === 0) return index;
  }
  throw new Error(`Cannot find matching ${closeCharacter}`);
}

function splitTopLevel(source, separator) {
  const entries = [];
  let start = 0;
  let angleDepth = 0;
  let braceDepth = 0;
  let parenthesisDepth = 0;
  for (let index = 0; index < source.length; index += 1) {
    const character = source[index];
    if (character === '<') angleDepth += 1;
    if (character === '>') angleDepth -= 1;
    if (character === '{') braceDepth += 1;
    if (character === '}') braceDepth -= 1;
    if (character === '(') parenthesisDepth += 1;
    if (character === ')') parenthesisDepth -= 1;
    if (
      character === separator &&
      angleDepth === 0 &&
      braceDepth === 0 &&
      parenthesisDepth === 0
    ) {
      entries.push(source.slice(start, index));
      start = index + 1;
    }
  }
  entries.push(source.slice(start));
  return entries;
}

function parseRustParameter(parameter) {
  const separator = parameter.indexOf(':');
  if (separator < 0) throw new Error(`Cannot parse Rust command parameter: ${parameter}`);
  const name = parameter.slice(0, separator).trim();
  const type = parameter.slice(separator + 1).trim();
  if (/\bState\s*</.test(type) || /(?:^|::)(?:AppHandle|Window|WebviewWindow)\b/.test(type)) {
    return null;
  }
  const optional = type.startsWith('Option<');
  return `${snakeToCamel(name)}${optional ? '?' : ''}:${normalizeRustType(type)}`;
}

function normalizeRustType(type) {
  const compact = type.replace(/\s+/g, '');
  if (compact === '()') return 'null';
  if (compact === 'String' || compact === '&str') return 'string';
  if (compact === 'bool') return 'boolean';
  if (/^u(?:8|16|32|64|128|size)$/.test(compact) || /^i(?:8|16|32|64|128|size)$/.test(compact)) {
    return 'number';
  }
  const vector = unwrapGeneric(compact, 'Vec');
  if (vector !== null) return `${normalizeRustType(vector)}[]`;
  const option = unwrapGeneric(compact, 'Option');
  if (option !== null) return `${normalizeRustType(option)} | null`;
  return compact.split('::').at(-1).replace(/Dto$/, '');
}

function unwrapGeneric(type, outer) {
  const prefix = `${outer}<`;
  return type.startsWith(prefix) && type.endsWith('>') ? type.slice(prefix.length, -1) : null;
}

function snakeToCamel(value) {
  return value.replace(/_([a-z])/g, (_, character) => character.toUpperCase());
}

function extractPropertyType(body, propertyName) {
  const match = new RegExp(`(?:^|;)\\s*${propertyName}\\s*:\\s*`).exec(body);
  if (!match) throw new Error(`Cannot find TypeScript ${propertyName} property`);
  const start = match.index + match[0].length;
  let braceDepth = 0;
  for (let index = start; index < body.length; index += 1) {
    if (body[index] === '{') braceDepth += 1;
    if (body[index] === '}') braceDepth -= 1;
    if (body[index] === ';' && braceDepth === 0) return body.slice(start, index).trim();
  }
  const remainder = body.slice(start).trim();
  if (remainder) return remainder;
  throw new Error(
    `Cannot parse TypeScript ${propertyName} property in ${JSON.stringify(body.trim())}`,
  );
}

function parseTypeScriptArguments(type) {
  if (type === 'undefined') return [];
  if (!type.startsWith('{') || !type.endsWith('}')) {
    throw new Error(`Command args must be an object or undefined, received ${type}`);
  }
  return splitTopLevel(type.slice(1, -1), ';')
    .map((entry) => entry.trim())
    .filter(Boolean)
    .map((entry) => {
      const match = entry.match(/^([a-zA-Z0-9_]+)(\?)?\s*:\s*(.+)$/);
      if (!match) throw new Error(`Cannot parse TypeScript command argument: ${entry}`);
      return `${match[1]}${match[2] ?? ''}:${normalizeTypeScriptType(match[3])}`;
    });
}

function normalizeTypeScriptType(type) {
  return type.replace(/\s+/g, ' ').trim();
}

const currentFile = fileURLToPath(import.meta.url);
if (path.resolve(process.argv[1] ?? '') === currentFile) {
  const rootDir = path.resolve(path.dirname(currentFile), '../..');
  try {
    verifyIpcContract(rootDir);
    process.stdout.write('Rust commands/events and TypeScript IPC maps are in parity.\n');
  } catch (error) {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  }
}
