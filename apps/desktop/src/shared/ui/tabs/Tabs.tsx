import React, { createContext, useContext, useId, useState, type KeyboardEvent } from 'react';

interface TabsContextValue {
  value: string;
  onValueChange: (value: string) => void;
  orientation?: 'horizontal' | 'vertical';
  variant?: 'default' | 'compact';
  fullWidth?: boolean;
  getTriggerId: (value: string) => string;
  getContentId: (value: string) => string;
}

const TabsContext = createContext<TabsContextValue | null>(null);

export interface TabsProps extends React.HTMLAttributes<HTMLDivElement> {
  value?: string;
  defaultValue?: string;
  onValueChange?: (value: string) => void;
  orientation?: 'horizontal' | 'vertical';
  variant?: 'default' | 'compact';
  fullWidth?: boolean;
}

export const Tabs = React.forwardRef<HTMLDivElement, TabsProps>(
  (
    {
      value,
      defaultValue,
      onValueChange,
      orientation = 'horizontal',
      variant = 'default',
      fullWidth = false,
      className = '',
      children,
      ...props
    },
    ref,
  ) => {
    const [internalValue, setInternalValue] = useState(defaultValue || '');
    const generatedId = useId();
    const isControlled = value !== undefined;
    const selectedValue = isControlled ? value : internalValue;
    const getSafeValue = (tabValue: string) => tabValue.replace(/[^a-zA-Z0-9_-]/g, '-');

    const handleValueChange = (newValue: string) => {
      onValueChange?.(newValue);
      if (!isControlled) setInternalValue(newValue);
    };

    return (
      <TabsContext.Provider
        value={{
          value: selectedValue,
          onValueChange: handleValueChange,
          orientation,
          variant,
          fullWidth,
          getTriggerId: (tabValue) => `${generatedId}-tab-${getSafeValue(tabValue)}`,
          getContentId: (tabValue) => `${generatedId}-panel-${getSafeValue(tabValue)}`,
        }}
      >
        <div ref={ref} className={className} {...props}>
          {children}
        </div>
      </TabsContext.Provider>
    );
  },
);
Tabs.displayName = 'Tabs';

export interface TabsListProps extends React.HTMLAttributes<HTMLDivElement> {
  fullWidth?: boolean;
}

export const TabsList = React.forwardRef<HTMLDivElement, TabsListProps>(
  ({ className = '', fullWidth, ...props }, ref) => {
    const ctx = useContext(TabsContext);

    // Keyboard navigation
    const handleKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
      const target = e.target as HTMLElement;
      if (target.getAttribute('role') !== 'tab') return;

      const tabList = target.closest('[role="tablist"]');
      if (!tabList) return;

      const tabs = Array.from(
        tabList.querySelectorAll('[role="tab"]:not([disabled])'),
      ) as HTMLElement[];
      const index = tabs.indexOf(target);
      if (index === -1) return;

      let nextIndex = index;
      if (e.key === 'ArrowRight' || e.key === 'ArrowDown') {
        e.preventDefault();
        nextIndex = (index + 1) % tabs.length;
      } else if (e.key === 'ArrowLeft' || e.key === 'ArrowUp') {
        e.preventDefault();
        nextIndex = (index - 1 + tabs.length) % tabs.length;
      } else if (e.key === 'Home') {
        e.preventDefault();
        nextIndex = 0;
      } else if (e.key === 'End') {
        e.preventDefault();
        nextIndex = tabs.length - 1;
      }

      if (nextIndex !== index) {
        tabs[nextIndex].focus();
        tabs[nextIndex].click(); // Automatically activate on focus change
      }
    };

    const variantClasses = ctx?.variant === 'compact' ? 'p-0.5 space-x-1' : 'p-1 space-x-1';
    const widthClass = fullWidth || ctx?.fullWidth ? 'flex w-full' : 'inline-flex';

    return (
      <div
        ref={ref}
        role="tablist"
        aria-orientation={ctx?.orientation || 'horizontal'}
        onKeyDown={handleKeyDown}
        className={`${widthClass} items-center justify-center rounded-lg bg-surface border border-border text-muted ${variantClasses} ${className}`}
        {...props}
      />
    );
  },
);
TabsList.displayName = 'TabsList';

export interface TabsTriggerProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  value: string;
}

export const TabsTrigger = React.forwardRef<HTMLButtonElement, TabsTriggerProps>(
  ({ value, className = '', disabled, ...props }, ref) => {
    const ctx = useContext(TabsContext);
    const isSelected = ctx?.value === value;

    const variantClasses = ctx?.variant === 'compact' ? 'px-3 py-1 text-xs' : 'px-3 py-1.5 text-sm';
    const widthClass = ctx?.fullWidth ? 'flex-1' : '';

    return (
      <button
        ref={ref}
        type="button"
        role="tab"
        id={ctx?.getTriggerId(value)}
        aria-selected={isSelected}
        aria-controls={ctx?.getContentId(value)}
        disabled={disabled}
        onClick={() => {
          if (!disabled) ctx?.onValueChange(value);
        }}
        tabIndex={isSelected ? 0 : -1}
        className={`motion-control inline-flex items-center justify-center whitespace-nowrap rounded-md border border-transparent font-medium focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-bg disabled:pointer-events-none disabled:opacity-60 disabled:saturate-50 ${variantClasses} ${widthClass} ${
          isSelected
            ? 'border-primary-action bg-primary-action text-primary-foreground shadow-sm'
            : 'hover:border-border hover:bg-secondary hover:text-text'
        } ${className}`}
        {...props}
      />
    );
  },
);
TabsTrigger.displayName = 'TabsTrigger';

export interface TabsContentProps extends React.HTMLAttributes<HTMLDivElement> {
  value: string;
}

export const TabsContent = React.forwardRef<HTMLDivElement, TabsContentProps>(
  ({ value, className = '', ...props }, ref) => {
    const ctx = useContext(TabsContext);
    const isSelected = ctx?.value === value;

    return (
      <div
        ref={ref}
        id={ctx?.getContentId(value)}
        role="tabpanel"
        aria-labelledby={ctx?.getTriggerId(value)}
        tabIndex={0}
        hidden={!isSelected}
        className={`mt-2 animate-content-in focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-focus focus-visible:ring-offset-2 focus-visible:ring-offset-bg ${className}`}
        {...props}
      />
    );
  },
);
TabsContent.displayName = 'TabsContent';
