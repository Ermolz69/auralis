import { useState } from 'react';
import { Button } from '../button';
import { Select } from './Select';

export function ControlledChoice() {
  const [value, setValue] = useState('0');
  const [pending, setPending] = useState('');
  const [requests, setRequests] = useState(0);
  return (
    <div className="flex max-w-sm flex-col gap-3">
      <Button>Before choice</Button>
      <Select
        label="Accepted source"
        value={value}
        helperText="The owner accepts a requested change."
        placeholder="Choose a source"
        optionGroups={[
          {
            label: 'Available',
            options: [
              { value: 0, label: 'Local video' },
              { value: 'subtitles', label: 'Subtitles' },
            ],
          },
          {
            label: 'Unavailable',
            options: [{ value: 'missing', label: 'Missing source', disabled: true }],
          },
        ]}
        onChange={(event) => {
          setPending(event.target.value);
          setRequests((count) => count + 1);
        }}
      />
      <Button
        disabled={!pending}
        onClick={() => {
          setValue(pending);
          setPending('');
        }}
      >
        Accept choice
      </Button>
      <p role="status" aria-label="Requested source">
        {pending || 'None'}
      </p>
      <p role="status" aria-label="Change requests">
        {requests}
      </p>
    </div>
  );
}

export function SelectionForm() {
  const [invalid, setInvalid] = useState(false);
  const [submitted, setSubmitted] = useState('No submission');
  return (
    <form
      className="flex max-w-sm flex-col gap-3"
      onSubmit={(event) => {
        event.preventDefault();
        setSubmitted(String(new FormData(event.currentTarget).get('source')));
      }}
      onReset={() => {
        setInvalid(false);
        setSubmitted('No submission');
      }}
    >
      <Select
        label="Required source"
        name="source"
        required
        placeholder="Choose a source"
        options={[
          { value: 'video', label: 'Video' },
          { value: 'missing', label: 'Missing file', disabled: true },
          { value: 'text', label: 'Text' },
        ]}
        helperText="Select an available source."
        error={invalid}
        errorText={invalid ? 'A source is required.' : undefined}
        onInvalid={() => setInvalid(true)}
        onChange={() => setInvalid(false)}
      />
      <Select
        label="Disabled source"
        disabled
        defaultValue="locked"
        options={[{ value: 'locked', label: 'Locked source' }]}
      />
      <Button type="submit">Continue</Button>
      <Button type="reset">Reset choice</Button>
      <p role="status" aria-label="Submitted source">
        {submitted}
      </p>
    </form>
  );
}
