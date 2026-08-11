import React, { useId } from 'react';

export interface InputProps extends React.InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  helperText?: string;
  errorText?: string;
  error?: boolean;
  leftIcon?: React.ReactNode;
  rightIcon?: React.ReactNode;
}

export const Input = React.forwardRef<HTMLInputElement, InputProps>(
  (
    {
      label,
      helperText,
      errorText,
      error = false,
      leftIcon,
      rightIcon,
      className = '',
      id,
      disabled,
      ...props
    },
    ref,
  ) => {
    const generatedId = useId();
    const inputId = id || generatedId;
    const resolvedErrorText = errorText || (error ? helperText : undefined);
    const resolvedHelperText = errorText ? helperText : error ? undefined : helperText;
    const helperId = resolvedHelperText ? `${inputId}-helper` : undefined;
    const errorId = resolvedErrorText ? `${inputId}-error` : undefined;
    const describedBy = [helperId, errorId].filter(Boolean).join(' ') || undefined;

    const baseWrapper = 'flex flex-col gap-1.5 w-full';

    // The input base wrapper styles.
    // If error -> border-danger
    // If focused -> ring-primary or border-primary
    const inputBase =
      'flex w-full items-center rounded-md border bg-surface-raised text-sm text-text transition-colors outline-none';

    const inputBorder = error
      ? 'border-danger focus-within:ring-2 focus-within:ring-danger focus-within:ring-offset-2 focus-within:ring-offset-bg focus-within:border-danger'
      : 'border-border hover:border-border-strong focus-within:border-focus';

    const inputDisabled = disabled ? 'opacity-50 cursor-not-allowed bg-bg' : '';

    return (
      <div className={`${baseWrapper} ${className}`}>
        {label && (
          <label htmlFor={inputId} className="text-xs font-medium text-muted">
            {label}
          </label>
        )}
        <div className={`relative ${inputBase} ${inputBorder} ${inputDisabled}`}>
          {leftIcon && (
            <div className="absolute left-3 flex items-center text-muted shrink-0 pointer-events-none">
              {leftIcon}
            </div>
          )}
          <input
            id={inputId}
            ref={ref}
            disabled={disabled}
            aria-invalid={error || undefined}
            aria-describedby={describedBy}
            aria-errormessage={errorId}
            className={`w-full bg-transparent py-2 outline-none placeholder:text-subtle ${
              leftIcon ? 'pl-9' : 'pl-3'
            } ${rightIcon ? 'pr-9' : 'pr-3'}`}
            {...props}
          />
          {rightIcon && (
            <div className="absolute right-3 flex items-center text-muted shrink-0 pointer-events-none">
              {rightIcon}
            </div>
          )}
        </div>
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

Input.displayName = 'Input';
