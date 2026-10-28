import { Icon } from '../icon';

export const toastAppearance = {
  default: {
    className: 'border-muted/35 from-surface-raised to-surface text-muted',
    label: 'Информация',
    icon: <Icon name="Info" size="md" color="muted" />,
  },
  success: {
    className: 'border-success/35 from-success-soft/75 to-surface text-success',
    label: 'Готово',
    icon: <Icon name="CircleCheck" size="md" color="success" />,
  },
  warning: {
    className: 'border-warning/35 from-warning-soft/75 to-surface text-warning',
    label: 'Обратите внимание',
    icon: <Icon name="TriangleAlert" size="md" color="warning" />,
  },
  danger: {
    className: 'border-danger/35 from-danger-soft/75 to-surface text-danger',
    label: 'Ошибка',
    icon: <Icon name="OctagonX" size="md" color="danger" />,
  },
};
