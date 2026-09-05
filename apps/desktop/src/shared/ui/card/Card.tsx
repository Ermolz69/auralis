import React from 'react';

// Card
export interface CardProps extends React.HTMLAttributes<HTMLDivElement> {
  variant?: 'default' | 'elevated' | 'interactive' | 'muted';
}
export const Card = React.forwardRef<HTMLDivElement, CardProps>(
  ({ className = '', variant = 'default', ...props }, ref) => {
    const base = 'rounded-md text-text';
    const variants = {
      default: 'border border-border bg-surface-raised',
      elevated: 'border border-border-strong bg-surface-raised shadow-md',
      interactive:
        'motion-surface cursor-pointer border border-border bg-surface-raised hover:-translate-y-px hover:border-border-strong hover:bg-surface-hover hover:shadow-md active:translate-y-0',
      muted: 'border border-border bg-surface',
    };
    return <div ref={ref} className={`${base} ${variants[variant]} ${className}`} {...props} />;
  },
);
Card.displayName = 'Card';

export const CardHeader = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(
  ({ className = '', ...props }, ref) => (
    <div ref={ref} className={`flex flex-col space-y-1 p-4 ${className}`} {...props} />
  ),
);
CardHeader.displayName = 'CardHeader';

export const CardTitle = React.forwardRef<
  HTMLHeadingElement,
  React.HTMLAttributes<HTMLHeadingElement>
>(({ className = '', ...props }, ref) => (
  <h3
    ref={ref}
    className={`text-sm font-semibold leading-none tracking-tight ${className}`}
    {...props}
  />
));
CardTitle.displayName = 'CardTitle';

export const CardDescription = React.forwardRef<
  HTMLParagraphElement,
  React.HTMLAttributes<HTMLParagraphElement>
>(({ className = '', ...props }, ref) => (
  <p ref={ref} className={`text-xs text-muted ${className}`} {...props} />
));
CardDescription.displayName = 'CardDescription';

export const CardContent = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(
  ({ className = '', ...props }, ref) => (
    <div ref={ref} className={`p-4 pt-0 ${className}`} {...props} />
  ),
);
CardContent.displayName = 'CardContent';

export const CardFooter = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(
  ({ className = '', ...props }, ref) => (
    <div ref={ref} className={`flex items-center p-4 pt-0 ${className}`} {...props} />
  ),
);
CardFooter.displayName = 'CardFooter';
