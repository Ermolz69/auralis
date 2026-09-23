export type Validator = (value: unknown) => boolean;

export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

export function isString(value: unknown): value is string {
  return typeof value === 'string';
}

export function isBoolean(value: unknown): value is boolean {
  return typeof value === 'boolean';
}

export function isNull(value: unknown): value is null {
  return value === null;
}

export function isFiniteNumber(value: unknown): value is number {
  return typeof value === 'number' && Number.isFinite(value);
}

export function isPositiveFiniteNumber(value: unknown): value is number {
  return isFiniteNumber(value) && value > 0;
}

export function isNonNegativeSafeInteger(value: unknown): value is number {
  return typeof value === 'number' && Number.isSafeInteger(value) && value >= 0;
}

export function isTimestamp(value: unknown): value is string {
  return isString(value) && Number.isFinite(Date.parse(value));
}

export function isEnumValue(value: unknown, values: ReadonlySet<string>): value is string {
  return isString(value) && values.has(value);
}

export function isNullable(value: unknown, validator: Validator): boolean {
  return value === null || validator(value);
}

export function nullableField(
  record: Record<string, unknown>,
  key: string,
  validator: Validator,
): boolean {
  return !Object.hasOwn(record, key) || record[key] === null || validator(record[key]);
}

export function isArrayOf(value: unknown, validator: Validator): boolean {
  return Array.isArray(value) && value.every(validator);
}
