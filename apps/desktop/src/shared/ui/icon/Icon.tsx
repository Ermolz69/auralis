import React from 'react';
import { icons } from './registry';

export type IconName = keyof typeof icons;

export interface IconProps extends Omit<React.SVGAttributes<SVGSVGElement>, 'color'> {
  name: IconName;
  size?: 'sm' | 'md' | 'lg' | number;
  color?: 'default' | 'primary' | 'muted' | 'danger' | 'success' | 'warning' | 'accent' | string;
  className?: string;
  ariaLabel?: string;
}

export const Icon = React.forwardRef<SVGSVGElement, IconProps>(
  ({ name, size = 'md', color = 'default', className = '', ariaLabel, ...props }, ref) => {
    const requestedIcon = icons[name];
    const LucideIcon = (requestedIcon ?? icons.CircleQuestionMark) as React.ElementType;

    if (!requestedIcon) {
      console.warn(`Icon "${name}" does not exist in lucide-react.`);
    }

    const sizeMap = {
      sm: 16,
      md: 20,
      lg: 24,
    };

    const iconSize = typeof size === 'number' ? size : sizeMap[size] || sizeMap.md;

    // We can just set the CSS color property via a class or style, as lucide uses currentColor for stroke
    const colorClassMap: Record<string, string> = {
      default: '',
      primary: 'text-primary',
      muted: 'text-muted',
      danger: 'text-danger',
      success: 'text-success',
      warning: 'text-warning',
      accent: 'text-accent',
    };

    const isPresetColor = color in colorClassMap;
    const colorClass = isPresetColor ? colorClassMap[color] : '';
    const customColorStyle = !isPresetColor && color !== 'default' ? { color } : undefined;

    return (
      <LucideIcon
        ref={ref}
        size={iconSize}
        className={`shrink-0 ${colorClass} ${className}`}
        style={{ ...customColorStyle, ...props.style }}
        role={ariaLabel ? 'img' : undefined}
        aria-hidden={ariaLabel ? undefined : true}
        aria-label={ariaLabel}
        {...props}
      />
    );
  },
);

Icon.displayName = 'Icon';
