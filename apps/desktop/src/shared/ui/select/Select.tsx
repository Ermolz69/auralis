import React, { useId } from 'react';

export interface SelectOption {
  value: string | number;
  label: string;
  disabled?: boolean;
}

export interface SelectOptionGroup {
  label: string;
  options: SelectOption[];
}

export interface SelectProps extends React.SelectHTMLAttributes<HTMLSelectElement> {
  label?: string;
  helperText?: string;
  errorText?: string;
  error?: boolean;
  options?: SelectOption[];
  optionGroups?: SelectOptionGroup[];
  placeholder?: string;
}

export const Select = React.forwardRef<HTMLSelectElement, SelectProps>(
  (
    {
      label,
      helperText,
      errorText,
      error = false,
      options = [],
      optionGroups = [],
      placeholder,
      className = '',
      id,
      disabled,
      ...props
    },
    ref,
  ) => {
    const generatedId = useId();
    const selectId = id || generatedId;
    const resolvedErrorText = errorText || (error ? helperText : undefined);
    const resolvedHelperText = errorText ? helperText : error ? undefined : helperText;
    const helperId = resolvedHelperText ? `${selectId}-helper` : undefined;
    const errorId = resolvedErrorText ? `${selectId}-error` : undefined;
    const describedBy = [helperId, errorId].filter(Boolean).join(' ') || undefined;

    const baseWrapper = 'flex flex-col gap-1.5 w-full';

    // appearance-none hides the default browser dropdown arrow
    // We add a custom SVG arrow in the wrapper below
    const selectBase =
      'flex w-full items-center bg-surface border rounded-md text-text text-sm transition-all outline-none appearance-none px-3 py-2 pr-10 cursor-pointer';

    const selectBorder = error
      ? 'border-danger focus:ring-2 focus:ring-danger focus:ring-offset-2 focus:ring-offset-bg focus:border-danger'
      : 'border-border hover:border-muted focus:border-focus focus:ring-2 focus:ring-focus focus:ring-offset-2 focus:ring-offset-bg';

    const selectDisabled = disabled ? 'opacity-50 cursor-not-allowed bg-bg' : '';

    return (
      <div className={`${baseWrapper} ${className}`}>
        {label && (
          <label htmlFor={selectId} className="text-sm font-medium text-text">
            {label}
          </label>
        )}
        <div className="relative w-full">
          <select
            id={selectId}
            ref={ref}
            disabled={disabled}
            aria-invalid={error || undefined}
            aria-describedby={describedBy}
            aria-errormessage={errorId}
            className={`${selectBase} ${selectBorder} ${selectDisabled}`}
            defaultValue={
              placeholder && !props.value && !props.defaultValue ? '' : props.defaultValue
            }
            {...props}
          >
            {placeholder && (
              <option value="" disabled hidden className="text-muted">
                {placeholder}
              </option>
            )}
            {options.map((opt) => (
              <option
                key={opt.value}
                value={opt.value}
                disabled={opt.disabled}
                className="bg-surface text-text"
              >
                {opt.label}
              </option>
            ))}
            {optionGroups.map((group) => (
              <optgroup key={group.label} label={group.label} className="bg-surface text-muted">
                {group.options.map((opt) => (
                  <option
                    key={opt.value}
                    value={opt.value}
                    disabled={opt.disabled}
                    className="bg-surface text-text"
                  >
                    {opt.label}
                  </option>
                ))}
              </optgroup>
            ))}
          </select>
          <div className="absolute right-3 top-1/2 -translate-y-1/2 pointer-events-none text-muted">
            <svg
              aria-hidden="true"
              xmlns="http://www.w3.org/2000/svg"
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
            >
              <polyline points="6 9 12 15 18 9"></polyline>
            </svg>
          </div>
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

Select.displayName = 'Select';
