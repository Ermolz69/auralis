import React, { useId } from 'react';

export interface TextareaProps extends React.TextareaHTMLAttributes<HTMLTextAreaElement> {
  label?: string;
  helperText?: string;
  error?: boolean;
  resizable?: boolean;
}

export const Textarea = React.forwardRef<HTMLTextAreaElement, TextareaProps>(
  (
    { label, helperText, error = false, resizable = true, className = '', id, disabled, ...props },
    ref,
  ) => {
    const generatedId = useId();
    const textareaId = id || generatedId;

    const baseWrapper = 'flex flex-col gap-1.5 w-full';

    // The textarea base styles.
    // If error -> border-danger
    // If focused -> ring-primary or border-primary
    const textareaBase =
      'flex w-full bg-surface border rounded-md text-text text-sm transition-all outline-none p-3 placeholder:text-muted';

    const textareaBorder = error
      ? 'border-danger focus:ring-2 focus:ring-danger focus:ring-offset-2 focus:ring-offset-bg focus:border-danger'
      : 'border-muted hover:border-muted/80 focus:border-primary focus:ring-2 focus:ring-primary focus:ring-offset-2 focus:ring-offset-bg';

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
          className={`${textareaBase} ${textareaBorder} ${textareaDisabled} ${resizeClass}`}
          {...props}
        />
        {(helperText || error) && (
          <span className={`text-xs ${error ? 'text-danger' : 'text-muted'}`}>{helperText}</span>
        )}
      </div>
    );
  },
);

Textarea.displayName = 'Textarea';
