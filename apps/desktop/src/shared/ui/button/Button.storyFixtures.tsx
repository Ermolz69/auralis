import { useState } from 'react';
import { Button } from './Button';

export function ButtonActions({ pending = false }: { pending?: boolean }) {
  const [activations, setActivations] = useState(0);
  const [submissions, setSubmissions] = useState(0);
  const [loading, setLoading] = useState(false);
  return (
    <form
      className="flex flex-wrap items-center gap-3"
      onSubmit={(event) => {
        event.preventDefault();
        setSubmissions((value) => value + 1);
      }}
    >
      <Button variant="secondary">Before actions</Button>
      <Button
        loading={loading}
        onClick={() => {
          setActivations((value) => value + 1);
          if (pending) setLoading(true);
        }}
      >
        Run action
      </Button>
      <Button disabled onClick={() => setActivations((value) => value + 1)}>
        Disabled action
      </Button>
      <Button type="submit">Submit form</Button>
      {pending && <Button onClick={() => setLoading(false)}>Complete request</Button>}
      <p role="status" aria-label="Action count">
        {activations}
      </p>
      <p role="status" aria-label="Submission count">
        {submissions}
      </p>
    </form>
  );
}
