import React, { useId } from 'react';

export interface InputProps extends React.InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  helperText?: string;
  error?: boolean;
  leftIcon?: React.ReactNode;
  rightIcon?: React.ReactNode;
}

export const Input = React.forwardRef<HTMLInputElement, InputProps>(
  (
    { label, helperText, error = false, leftIcon, rightIcon, className = '', id, disabled, ...props },
    ref
  ) => {
    const generatedId = useId();
    const inputId = id || generatedId;

    const baseWrapper = 'flex flex-col gap-1.5 w-full';
    
    // The input base wrapper styles.
    // If error -> border-danger
    // If focused -> ring-primary or border-primary
    const inputBase =
      'flex w-full items-center bg-surface border rounded-md text-text text-sm transition-all outline-none';
    
    const inputBorder = error
      ? 'border-danger focus-within:ring-1 focus-within:ring-danger focus-within:border-danger'
      : 'border-muted hover:border-muted/80 focus-within:border-primary focus-within:ring-1 focus-within:ring-primary';

    const inputDisabled = disabled ? 'opacity-50 cursor-not-allowed bg-bg' : '';

    return (
      <div className={`${baseWrapper} ${className}`}>
        {label && (
          <label htmlFor={inputId} className="text-sm font-medium text-text">
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
            className={`w-full bg-transparent outline-none placeholder:text-muted py-2 ${
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
        {(helperText || error) && (
          <span className={`text-xs ${error ? 'text-danger' : 'text-muted'}`}>
            {helperText}
          </span>
        )}
      </div>
    );
  }
);

Input.displayName = 'Input';
