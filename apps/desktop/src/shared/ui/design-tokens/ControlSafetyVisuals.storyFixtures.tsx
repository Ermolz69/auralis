import { Badge, Button, Card, Icon, Notice, Progress, StateView } from '@/shared/ui';
import { icons } from '../icon/registry';

const buttonVariants = ['primary', 'secondary', 'ghost', 'danger'] as const;
const buttonSizes = ['sm', 'md', 'lg'] as const;
const badgeVariants = [
  'default',
  'primary',
  'accent',
  'success',
  'warning',
  'danger',
  'muted',
] as const;
const badgeSizes = ['sm', 'md'] as const;
const progressVariants = ['default', 'success', 'warning', 'danger'] as const;
const iconSizes = ['sm', 'md', 'lg'] as const;
const iconColors = [
  'default',
  'primary',
  'muted',
  'danger',
  'success',
  'warning',
  'accent',
] as const;
const iconNames = Object.keys(icons) as Array<keyof typeof icons>;

export const CONTROL_SAFETY_ICON_COUNT = iconNames.length;

export function ControlSafetyVisuals() {
  return (
    <>
      <section aria-labelledby="control-safety-buttons" className="space-y-3">
        <h2 id="control-safety-buttons" className="text-base font-medium">
          Buttons and badges
        </h2>
        <div className="flex flex-wrap gap-2">
          {buttonVariants.flatMap((variant) =>
            buttonSizes.map((size) => (
              <Button
                key={`${variant}-${size}`}
                data-testid="safe-button"
                variant={variant}
                size={size}
              >
                {variant} {size}
              </Button>
            )),
          )}
          <Button data-testid="safe-button" loading>
            Loading
          </Button>
          <Button data-testid="safe-button" disabled fullWidth>
            Disabled full width
          </Button>
        </div>
        <div className="flex flex-wrap gap-2">
          {badgeVariants.flatMap((variant) =>
            badgeSizes.map((size) => (
              <Badge
                key={`${variant}-${size}`}
                data-testid="safe-badge"
                variant={variant}
                size={size}
              >
                {variant} {size}
              </Badge>
            )),
          )}
        </div>
      </section>

      <section aria-labelledby="control-safety-surfaces" className="space-y-3">
        <h2 id="control-safety-surfaces" className="text-base font-medium">
          Surfaces and status
        </h2>
        <div className="grid gap-3 md:grid-cols-2">
          {(['default', 'elevated', 'interactive', 'muted'] as const).map((variant) => (
            <Card
              key={variant}
              data-testid="safe-card"
              variant={variant}
              className="p-3"
              {...(variant === 'interactive' ? { role: 'button', tabIndex: 0 } : {})}
            >
              {variant} card
            </Card>
          ))}
          {(['neutral', 'accent', 'warning', 'danger'] as const).map((tone) => (
            <div key={tone} data-testid="safe-notice">
              <Notice icon="Info" tone={tone} title={`${tone} notice`}>
                Safe finite tone option.
              </Notice>
            </div>
          ))}
          {(['neutral', 'danger'] as const).flatMap((tone) =>
            (['default', 'compact'] as const).map((density) => (
              <div key={`${tone}-${density}`} data-testid="safe-state-view">
                <StateView
                  icon="Inbox"
                  title={`${tone} ${density}`}
                  description="Safe state-view options."
                  tone={tone}
                  density={density}
                  loading={tone === 'neutral' && density === 'compact'}
                />
              </div>
            )),
          )}
        </div>
      </section>

      <section aria-labelledby="control-safety-icons" className="space-y-3">
        <h2 id="control-safety-icons" className="text-base font-medium">
          Registered icons
        </h2>
        <div className="flex flex-wrap gap-2">
          {iconNames.map((name, index) => (
            <Icon
              key={name}
              data-testid="safe-icon"
              name={name}
              size={iconSizes[index % iconSizes.length]}
              color={iconColors[index % iconColors.length]}
              ariaLabel={name}
            />
          ))}
        </div>
      </section>

      <section aria-labelledby="control-safety-progress" className="space-y-3">
        <h2 id="control-safety-progress" className="text-base font-medium">
          Progress boundaries
        </h2>
        {progressVariants.flatMap((variant) =>
          [0, 50, 100].map((value) => (
            <Progress
              key={`${variant}-${value}`}
              data-testid="safe-progress"
              value={value}
              variant={variant}
              label={`${variant} progress at ${value}%`}
            />
          )),
        )}
        <Progress data-testid="safe-progress" indeterminate label="Indeterminate progress" />
      </section>
    </>
  );
}
