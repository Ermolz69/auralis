import type { IconName } from '../../../shared/ui/icon';
import type { SelectOptionGroup } from '../../../shared/ui/select';
import { COLOR_THEMES } from '../../../shared/theme';

export const colorThemeOptionGroups: SelectOptionGroup[] = [
  {
    label: 'Light themes',
    options: COLOR_THEMES.filter((theme) => theme.appearance === 'light').map((theme) => ({
      value: theme.id,
      label: theme.label,
    })),
  },
  {
    label: 'Dark themes',
    options: COLOR_THEMES.filter((theme) => theme.appearance === 'dark').map((theme) => ({
      value: theme.id,
      label: theme.label,
    })),
  },
  {
    label: 'Custom themes',
    options: [
      {
        value: '__custom-themes-empty',
        label: 'No custom themes yet',
        disabled: true,
      },
    ],
  },
];

export const unavailableSections: Array<{
  label: string;
  description: string;
  detail: string;
  ariaLabel: string;
  icon: IconName;
}> = [
  {
    label: 'Export defaults',
    description: 'Output directory, resolution and format',
    detail: 'Export defaults are not part of the current app contract.',
    ariaLabel: 'Export defaults unavailable',
    icon: 'FolderOutput',
  },
];
