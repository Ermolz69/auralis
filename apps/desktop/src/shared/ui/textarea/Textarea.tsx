import React, { useId } from 'react';

export interface TextareaProps extends React.TextareaHTMLAttributes<HTMLTextAreaElement> {
  label?: string;
  helperText?: string;
  errorText?: string;
  error?: boolean;
  resizable?: boolean;
}

export const Textarea = React.forwardRef<HTMLTextAreaElement, TextareaProps>(
  (
    {
      label,
      helperText,
      errorText,
      error = false,
      resizable = true,
      className = '',
      id,
      disabled,
      ...props
    },
    ref,
  ) => {
    const generatedId = useId();
    const textareaId = id || generatedId;
    const resolvedErrorText = errorText || (error ? helperText : undefined);
    const resolvedHelperText = errorText ? helperText : error ? undefined : helperText;
    const helperId = resolvedHelperText ? `${textareaId}-helper` : undefined;
    const errorId = resolvedErrorText ? `${textareaId}-error` : undefined;
    const describedBy = [helperId, errorId].filter(Boolean).join(' ') || undefined;

    const baseWrapper = 'flex flex-col gap-1.5 w-full';

    // The textarea base styles.
    // If error -> border-danger
    // If focused -> ring-primary or border-primary
    const textareaBase =
      'flex w-full bg-surface border rounded-md text-text text-sm transition-all outline-none p-3 placeholder:text-muted';

    const textareaBorder = error
      ? 'border-danger focus:ring-2 focus:ring-danger focus:ring-offset-2 focus:ring-offset-bg focus:border-danger'
      : 'border-border hover:border-muted focus:border-focus focus:ring-2 focus:ring-focus focus:ring-offset-2 focus:ring-offset-bg';

    const textareaDisabled = disabled ? 'opacity-50 cursor-not-allowed bg-bg' : '';
    const resizeClass = resizable ? 'resize-y' : 'resize-none';

    return (
      <div className={`${baseWrapper} ${className}`}>
        {label && (
          <label htmlFor={textareaId} className="text-sm font-medium text-text">
            {label}
          </label>
        )}
        <textarea
          id={textareaId}
          ref={ref}
          disabled={disabled}
          aria-invalid={error || undefined}
          aria-describedby={describedBy}
          aria-errormessage={errorId}
          className={`${textareaBase} ${textareaBorder} ${textareaDisabled} ${resizeClass}`}
          {...props}
        />
        {resolvedHelperText && (
          <span id={helperId} className="text-xs text-muted">
            {resolvedHelperText}
          </span>
        )}
        {resolvedErrorText && (
          <span id={errorId} className="text-xs text-danger" role="alert">
            {resolvedErrorText}
          </span>
        )}
      </div>
    );
  },
);

Textarea.displayName = 'Textarea';
