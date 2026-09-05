export function FieldMessages({
  helperId,
  errorId,
  helperText,
  errorText,
}: {
  helperId?: string;
  errorId?: string;
  helperText?: string;
  errorText?: string;
}) {
  return (
    <>
      {helperText && (
        <span id={helperId} className="text-xs text-muted">
          {helperText}
        </span>
      )}
      {errorText && (
        <span id={errorId} className="text-xs text-danger" role="alert">
          {errorText}
        </span>
      )}
    </>
  );
}
