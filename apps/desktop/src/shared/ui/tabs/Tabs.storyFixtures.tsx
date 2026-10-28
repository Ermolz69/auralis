import { useState } from 'react';
import { Button } from '../button';
import { Input } from '../input';
import { Tabs, TabsContent, TabsList, TabsTrigger } from './Tabs';

export function ControlledTabs() {
  const [value, setValue] = useState('general');
  const [pending, setPending] = useState('');
  const [requests, setRequests] = useState(0);
  return (
    <div className="flex max-w-sm flex-col gap-3">
      <Tabs
        value={value}
        onValueChange={(next) => {
          setPending(next);
          setRequests((count) => count + 1);
        }}
      >
        <TabsList aria-label="Controlled views">
          <TabsTrigger value="general">General</TabsTrigger>
          <TabsTrigger value="advanced">Advanced</TabsTrigger>
        </TabsList>
        <TabsContent value="general">Accepted general view</TabsContent>
        <TabsContent value="advanced">Accepted advanced view</TabsContent>
      </Tabs>
      <Button
        disabled={!pending}
        onClick={() => {
          setValue(pending);
          setPending('');
        }}
      >
        Accept view
      </Button>
      <p role="status" aria-label="Requested view">
        {pending || 'None'}
      </p>
      <p role="status" aria-label="View requests">
        {requests}
      </p>
    </div>
  );
}

export function PersistentPanels() {
  return (
    <Tabs defaultValue="notes">
      <TabsList aria-label="Saved panels">
        <TabsTrigger value="notes">Notes</TabsTrigger>
        <TabsTrigger value="source">Source</TabsTrigger>
      </TabsList>
      <TabsContent value="notes">
        <Input label="Retained note" defaultValue="Initial note" />
      </TabsContent>
      <TabsContent value="source">Source information</TabsContent>
    </Tabs>
  );
}

export function VerticalPanels() {
  return (
    <Tabs defaultValue="first" orientation="vertical">
      <TabsList aria-label="Vertical views" className="flex-col">
        <TabsTrigger value="first">First view</TabsTrigger>
        <TabsTrigger value="unavailable" disabled>
          Unavailable view
        </TabsTrigger>
        <TabsTrigger value="last">Last view</TabsTrigger>
      </TabsList>
      <TabsContent value="first">First content</TabsContent>
      <TabsContent value="unavailable">Unavailable content</TabsContent>
      <TabsContent value="last">Last content</TabsContent>
    </Tabs>
  );
}
