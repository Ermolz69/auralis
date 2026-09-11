import { createContext } from 'react';

export interface DialogContextValue {
  handleClose: () => void;
  titleId: string;
  descriptionId: string;
  setHasDescription: (hasDescription: boolean) => void;
}

export const DialogContext = createContext<DialogContextValue | null>(null);
