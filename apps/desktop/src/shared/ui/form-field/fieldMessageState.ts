export type FieldMessageOptions = {
  helperText?: string;
  errorText?: string;
  error: boolean;
};

export function getFieldMessageState(controlId: string, options: FieldMessageOptions) {
  const resolvedErrorText = options.errorText || (options.error ? options.helperText : undefined);
  const resolvedHelperText = options.errorText
    ? options.helperText
    : options.error
      ? undefined
      : options.helperText;
  const helperId = resolvedHelperText ? `${controlId}-helper` : undefined;
  const errorId = resolvedErrorText ? `${controlId}-error` : undefined;

  return {
    helperId,
    errorId,
    describedBy: [helperId, errorId].filter(Boolean).join(' ') || undefined,
    resolvedHelperText,
    resolvedErrorText,
  };
}
