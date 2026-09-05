import React from 'react';

// Root container for a page. Ensures dark theme bg and min height.
export const Page = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(
  ({ className = '', ...props }, ref) => (
    <div ref={ref} className={`min-h-full w-full bg-bg text-text ${className}`} {...props} />
  ),
);
Page.displayName = 'Page';

export const PageTopBar = React.forwardRef<HTMLElement, React.HTMLAttributes<HTMLElement>>(
  ({ className = '', ...props }, ref) => (
    <header
      ref={ref}
      className={`flex shrink-0 items-start border-b border-border px-5 py-5 sm:px-8 ${className}`}
      {...props}
    />
  ),
);
PageTopBar.displayName = 'PageTopBar';

// Container that sets the max-width and padding.
export interface PageContainerProps extends React.HTMLAttributes<HTMLDivElement> {
  size?: 'sm' | 'md' | 'full';
}
export const PageContainer = React.forwardRef<HTMLDivElement, PageContainerProps>(
  ({ className = '', size = 'md', ...props }, ref) => {
    const sizes = {
      sm: 'max-w-3xl',
      md: 'max-w-5xl',
      full: 'max-w-full',
    };
    return (
      <div
        ref={ref}
        className={`mx-auto flex min-h-full flex-col gap-6 px-4 py-5 sm:px-6 lg:px-8 ${sizes[size]} ${className}`}
        {...props}
      />
    );
  },
);
PageContainer.displayName = 'PageContainer';

export const PageHeader = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(
  ({ className = '', ...props }, ref) => (
    <div
      ref={ref}
      className={`flex flex-col sm:flex-row sm:items-start sm:justify-between gap-4 shrink-0 ${className}`}
      {...props}
    />
  ),
);
PageHeader.displayName = 'PageHeader';

export const PageHeaderGroup = React.forwardRef<
  HTMLDivElement,
  React.HTMLAttributes<HTMLDivElement>
>(({ className = '', ...props }, ref) => (
  <div ref={ref} className={`flex flex-col gap-1.5 ${className}`} {...props} />
));
PageHeaderGroup.displayName = 'PageHeaderGroup';

export const PageTitle = React.forwardRef<
  HTMLHeadingElement,
  React.HTMLAttributes<HTMLHeadingElement>
>(({ className = '', ...props }, ref) => (
  <h1
    ref={ref}
    className={`text-xl font-semibold tracking-tight text-text sm:text-2xl ${className}`}
    {...props}
  />
));
PageTitle.displayName = 'PageTitle';

export const PageDescription = React.forwardRef<
  HTMLParagraphElement,
  React.HTMLAttributes<HTMLParagraphElement>
>(({ className = '', ...props }, ref) => (
  <p ref={ref} className={`text-sm text-muted ${className}`} {...props} />
));
PageDescription.displayName = 'PageDescription';

export const PageActions = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(
  ({ className = '', ...props }, ref) => (
    <div ref={ref} className={`flex items-center gap-3 shrink-0 ${className}`} {...props} />
  ),
);
PageActions.displayName = 'PageActions';

export const PageContent = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(
  ({ className = '', ...props }, ref) => (
    <div ref={ref} className={`flex-1 flex flex-col gap-6 ${className}`} {...props} />
  ),
);
PageContent.displayName = 'PageContent';

// Layout variant that creates a two-column setup (main content + sidebar)
export const PageLayoutWithSidebar = React.forwardRef<
  HTMLDivElement,
  React.HTMLAttributes<HTMLDivElement>
>(({ className = '', ...props }, ref) => (
  <div
    ref={ref}
    className={`flex flex-col md:flex-row gap-8 lg:gap-12 w-full flex-1 ${className}`}
    {...props}
  />
));
PageLayoutWithSidebar.displayName = 'PageLayoutWithSidebar';

// Sidebar panel for the two-column layout
export const PageSidebar = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(
  ({ className = '', ...props }, ref) => (
    <aside
      ref={ref}
      className={`w-full md:w-64 lg:w-80 shrink-0 flex flex-col gap-4 ${className}`}
      {...props}
    />
  ),
);
PageSidebar.displayName = 'PageSidebar';

// Main content area for the two-column layout
export const PageSidebarContent = React.forwardRef<
  HTMLDivElement,
  React.HTMLAttributes<HTMLDivElement>
>(({ className = '', ...props }, ref) => (
  <div ref={ref} className={`flex-1 min-w-0 flex flex-col gap-6 ${className}`} {...props} />
));
PageSidebarContent.displayName = 'PageSidebarContent';
