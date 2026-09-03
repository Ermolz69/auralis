import { Badge } from '../../../shared/ui/badge';
import { Icon } from '../../../shared/ui/icon';
import { Page } from '../../../shared/ui/page-layout';
import { Select } from '../../../shared/ui/select';
import { COLOR_THEMES, isColorTheme, useColorTheme } from '../../../shared/theme';
import { colorThemeOptionGroups, unavailableSections } from './settings.data';

export const SettingsPage = () => {
  const { colorTheme, setColorTheme } = useColorTheme();
  const activeTheme = COLOR_THEMES.find((theme) => theme.id === colorTheme);

  return (
    <Page className="flex h-full min-h-0 flex-col overflow-hidden">
      <header className="shrink-0 border-b border-border px-5 py-5 sm:px-8">
        <h1 className="text-xl font-semibold text-text">Settings</h1>
        <p className="mt-0.5 text-xs text-muted">Workspace preferences</p>
      </header>

      <div className="min-h-0 flex-1 overflow-y-auto px-4 py-5 sm:px-8">
        <div className="w-full max-w-2xl space-y-3">
          <section className="rounded-md border border-border bg-surface-raised p-4">
            <div className="flex items-start gap-3">
              <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded-sm border border-border bg-surface text-primary">
                <Icon name="Palette" size={15} />
              </span>
              <div className="min-w-0 flex-1">
                <h2 className="text-sm font-semibold text-text">Appearance</h2>
                <p className="text-xs text-muted">Theme and interface colors</p>
              </div>
              <div className="flex gap-1" aria-hidden="true">
                <span className="h-3 w-3 rounded-full bg-primary" />
                <span className="h-3 w-3 rounded-full bg-accent" />
                <span className="h-3 w-3 rounded-full bg-success" />
              </div>
            </div>

            <div className="mt-4 border-t border-border/70 pt-4">
              <Select
                label="Color theme"
                value={colorTheme}
                optionGroups={colorThemeOptionGroups}
                helperText={activeTheme?.description}
                onChange={(event) => {
                  if (isColorTheme(event.target.value)) setColorTheme(event.target.value);
                }}
              />
            </div>
          </section>

          {unavailableSections.map((section) => (
            <section
              key={section.label}
              role="status"
              aria-label={section.ariaLabel}
              className="flex items-start gap-3 rounded-md border border-border bg-surface-raised p-4"
            >
              <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded-sm border border-border bg-surface text-muted">
                <Icon name={section.icon} size={15} />
              </span>
              <div className="min-w-0 flex-1">
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <div>
                    <h2 className="text-sm font-semibold text-text">{section.label}</h2>
                    <p className="text-xs text-muted">{section.description}</p>
                  </div>
                  <Badge variant="muted" size="sm">
                    Unavailable
                  </Badge>
                </div>
                <p className="mt-3 border-t border-border/70 pt-3 text-xs text-subtle">
                  {section.detail}
                </p>
              </div>
            </section>
          ))}

          <p className="pt-2 text-xs text-subtle">
            Цветовая тема хранится локально и применяется при следующем запуске приложения.
          </p>
        </div>
      </div>
    </Page>
  );
};
