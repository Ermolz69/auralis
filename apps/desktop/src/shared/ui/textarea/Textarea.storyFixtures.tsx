import { useState } from 'react';
import { Button } from '../button';
import { Textarea } from './Textarea';

export function NotesForm() {
  const [invalid, setInvalid] = useState(false);
  const [submitted, setSubmitted] = useState('No submission');
  return (
    <form
      className="flex max-w-sm flex-col gap-3"
      onSubmit={(event) => {
        event.preventDefault();
        setSubmitted(String(new FormData(event.currentTarget).get('notes')));
      }}
    >
      <Textarea
        label="Review notes"
        name="notes"
        required
        maxLength={18}
        helperText="Use several lines."
        error={invalid}
        errorText={invalid ? 'Notes are required.' : undefined}
        onInvalid={() => setInvalid(true)}
        onChange={() => setInvalid(false)}
        resizable={false}
      />
      <Button type="submit">Save notes</Button>
      <p role="status" aria-label="Saved notes">
        {submitted}
      </p>
    </form>
  );
}

export function NotesAvailability() {
  return (
    <div className="flex max-w-sm flex-col gap-3">
      <Button>Before notes</Button>
      <Textarea label="Disabled notes" disabled defaultValue="Locked notes" />
      <Textarea label="Read-only notes" readOnly defaultValue="Inspectable notes" />
      <Button>After notes</Button>
    </div>
  );
}
