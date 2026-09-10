import { Input, Select, Textarea } from '@/shared/ui';

const longText = `Boundary content ${'x'.repeat(512)}`;

export function ControlSafetyFields() {
  return (
    <section aria-labelledby="control-safety-fields" className="grid gap-4 md:grid-cols-2">
      <h2 id="control-safety-fields" className="col-span-full text-base font-medium">
        Field boundaries
      </h2>
      <Input data-testid="safe-input" label="Regular input" helperText="Helper text" />
      <Input data-testid="safe-input" label="Error input" error helperText="Fallback error text" />
      <Input data-testid="safe-input" label="Disabled input" disabled />
      <Input data-testid="safe-input" label={longText} defaultValue={longText} />
      <Textarea data-testid="safe-textarea" label="Regular textarea" resizable />
      <Textarea
        data-testid="safe-textarea"
        label="Error textarea"
        error
        helperText="Fallback error text"
      />
      <Textarea data-testid="safe-textarea" label="Disabled textarea" disabled />
      <Textarea data-testid="safe-textarea" label={longText} defaultValue={longText} />
      <Select
        data-testid="safe-select"
        label="Large finite option list"
        options={Array.from({ length: 250 }, (_, index) => ({
          value: index,
          label: `Option ${index + 1}`,
        }))}
      />
      <Select
        data-testid="safe-select"
        label="Disabled select"
        disabled
        options={[{ value: 'safe', label: 'Safe option' }]}
      />
    </section>
  );
}
