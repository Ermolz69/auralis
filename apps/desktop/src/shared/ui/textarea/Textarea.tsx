import React, { useId } from 'react';
import { FieldMessages, getFieldMessageState } from '../form-field';

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
    const { helperId, errorId, describedBy, resolvedHelperText, resolvedErrorText } =
      getFieldMessageState(textareaId, { helperText, errorText, error });

    const baseWrapper = 'flex flex-col gap-1.5 w-full';

    // The textarea base styles.
    // If error -> border-danger
    // If focused -> ring-primary or border-primary
    const textareaBase =
      'motion-field flex w-full rounded-md border bg-surface p-3 text-sm text-text outline-none placeholder:text-muted';

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
        <FieldMessages
          helperId={helperId}
          errorId={errorId}
          helperText={resolvedHelperText}
          errorText={resolvedErrorText}
        />
      </div>
    );
  },
);

Textarea.displayName = 'Textarea';
