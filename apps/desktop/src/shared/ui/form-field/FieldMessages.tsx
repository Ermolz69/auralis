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
        <span id={helperId} className="min-w-0 break-words text-xs text-muted">
          {helperText}
        </span>
      )}
      {errorText && (
        <span id={errorId} className="min-w-0 break-words text-xs text-danger" role="alert">
          {errorText}
        </span>
      )}
    </>
  );
}
