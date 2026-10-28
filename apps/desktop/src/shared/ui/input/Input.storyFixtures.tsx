import { useState } from 'react';
import { Button } from '../button';
import { Input } from './Input';

export function EmailValidation() {
  const [invalid, setInvalid] = useState(false);
  const [submitted, setSubmitted] = useState('No submission');
  return (
    <form
      className="flex max-w-sm flex-col gap-3"
      onSubmit={(event) => {
        event.preventDefault();
        setSubmitted(String(new FormData(event.currentTarget).get('email')));
      }}
    >
      <Input
        name="email"
        label="Delivery email"
        type="email"
        required
        helperText="Used for delivery updates."
        error={invalid}
        errorText={invalid ? 'Enter a valid email address.' : undefined}
        onInvalid={() => setInvalid(true)}
        onChange={() => setInvalid(false)}
      />
      <Button type="submit">Send update</Button>
      <p role="status" aria-label="Submitted email">
        {submitted}
      </p>
    </form>
  );
}

export function InputAvailability() {
  return (
    <div className="flex max-w-sm flex-col gap-3">
      <Button>Before fields</Button>
      <Input label="Disabled value" disabled defaultValue="Locked" />
      <Input label="Read-only value" readOnly defaultValue="Inspectable" />
      <Button>After fields</Button>
    </div>
  );
}
