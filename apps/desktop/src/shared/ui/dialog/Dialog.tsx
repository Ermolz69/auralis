import React, { createContext, useContext, useEffect, useRef, useState, useId } from 'react';
import { Icon } from '../icon';

interface DialogContextValue {
  handleClose: () => void;
  titleId: string;
  descriptionId: string;
  setHasDescription: (hasDescription: boolean) => void;
}

const DialogContext = createContext<DialogContextValue | null>(null);

export interface DialogProps {
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
  trigger?: React.ReactNode;
  children: React.ReactNode;
}

export const Dialog = ({ open, onOpenChange, trigger, children }: DialogProps) => {
  const [internalOpen, setInternalOpen] = useState(false);
  const dialogRef = useRef<HTMLDialogElement>(null);

  const titleId = useId();
  const descriptionId = useId();
  const [hasDescription, setHasDescription] = useState(false);

  const isControlled = open !== undefined;
  const isOpen = isControlled ? open : internalOpen;

  const handleOpen = () => {
    if (isControlled && onOpenChange) onOpenChange(true);
    else setInternalOpen(true);
  };

  const handleClose = () => {
    if (isControlled && onOpenChange) onOpenChange(false);
    else setInternalOpen(false);
  };

  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;

    if (isOpen && !dialog.open) {
      dialog.showModal();
      document.body.style.overflow = 'hidden';
    } else if (!isOpen && dialog.open) {
      dialog.close();
      document.body.style.overflow = '';
    }

    return () => {
      document.body.style.overflow = '';
    };
  }, [isOpen]);

  const handleCancel = (e: React.SyntheticEvent) => {
    e.preventDefault();
    handleClose();
  };

  const handleBackdropClick = (e: React.MouseEvent<HTMLDialogElement>) => {
    if (e.target === e.currentTarget) {
      handleClose();
    }
  };

  const triggerElement = React.isValidElement<{ onClick?: React.MouseEventHandler }>(trigger) ? (
    React.cloneElement(trigger, {
      onClick: (event) => {
        trigger.props.onClick?.(event);
        if (!event.defaultPrevented) handleOpen();
      },
    })
  ) : trigger ? (
    <button type="button" onClick={handleOpen}>
      {trigger}
    </button>
  ) : null;

  return (
    <DialogContext.Provider value={{ handleClose, titleId, descriptionId, setHasDescription }}>
      {triggerElement}
      <dialog
        ref={dialogRef}
        onCancel={handleCancel}
        onClick={handleBackdropClick}
        aria-labelledby={titleId}
        aria-describedby={hasDescription ? descriptionId : undefined}
        aria-modal="true"
        className="backdrop:bg-black/70 backdrop:backdrop-blur-sm m-auto rounded-xl bg-surface border border-border shadow-2xl p-0 text-text w-full max-w-lg open:animate-dialog-in focus:outline-none"
      >
        <div className="relative w-full h-full p-6">{children}</div>
      </dialog>
    </DialogContext.Provider>
  );
};

export const DialogHeader = ({
  className = '',
  children,
  ...props
}: React.HTMLAttributes<HTMLDivElement>) => (
  <div
    className={`flex flex-col space-y-1.5 text-center sm:text-left mb-4 ${className}`}
    {...props}
  >
    {children}
  </div>
);

export const DialogFooter = ({
  className = '',
  children,
  ...props
}: React.HTMLAttributes<HTMLDivElement>) => (
  <div
    className={`flex flex-col-reverse sm:flex-row sm:justify-end sm:space-x-2 mt-6 ${className}`}
    {...props}
  >
    {children}
  </div>
);

export const DialogTitle = ({
  className = '',
  children,
  ...props
}: React.HTMLAttributes<HTMLHeadingElement>) => {
  const ctx = useContext(DialogContext);
  return (
    <h2
      id={ctx?.titleId}
      className={`text-lg font-semibold leading-none tracking-tight ${className}`}
      {...props}
    >
      {children}
    </h2>
  );
};

export const DialogDescription = ({
  className = '',
  children,
  ...props
}: React.HTMLAttributes<HTMLParagraphElement>) => {
  const ctx = useContext(DialogContext);
  const setHasDescription = ctx?.setHasDescription;

  useEffect(() => {
    setHasDescription?.(true);
    return () => setHasDescription?.(false);
  }, [setHasDescription]);

  return (
    <p id={ctx?.descriptionId} className={`text-sm text-muted ${className}`} {...props}>
      {children}
    </p>
  );
};

export const DialogClose = ({
  className = '',
  ...props
}: React.ButtonHTMLAttributes<HTMLButtonElement>) => {
  const ctx = useContext(DialogContext);
  return (
    <button
      type="button"
      aria-label="Close dialog"
      onClick={ctx?.handleClose}
      className={`absolute right-4 top-4 rounded-md border border-transparent p-1 opacity-70 transition-all hover:border-border hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-focus focus:ring-offset-2 focus:ring-offset-surface disabled:pointer-events-none disabled:opacity-50 ${className}`}
      {...props}
    >
      <Icon name="X" size="sm" />
    </button>
  );
};

export const DialogCloseAction = ({ children }: { children: React.ReactNode }) => {
  const ctx = useContext(DialogContext);
  if (React.isValidElement<{ onClick?: React.MouseEventHandler }>(children)) {
    return React.cloneElement(children, {
      onClick: (event) => {
        children.props.onClick?.(event);
        if (!event.defaultPrevented) ctx?.handleClose();
      },
    });
  }

  return (
    <button type="button" onClick={ctx?.handleClose}>
      {children}
    </button>
  );
};
