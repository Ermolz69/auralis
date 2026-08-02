import {
  Page,
  PageContainer,
  PageHeader,
  PageHeaderGroup,
  PageTitle,
  PageDescription,
  PageContent,
} from '../../../shared/ui/page-layout';
import { Card, CardHeader, CardTitle, CardDescription, CardContent } from '../../../shared/ui/card';
import { Badge } from '../../../shared/ui/badge';

export const SettingsPage = () => {
  return (
    <Page className="overflow-y-auto min-h-screen">
      <PageContainer size="md" className="py-12">
        <PageHeader className="mb-8">
          <PageHeaderGroup>
            <PageTitle>Settings</PageTitle>
            <PageDescription>Available preferences and defaults will appear here.</PageDescription>
          </PageHeaderGroup>
        </PageHeader>

        <PageContent className="gap-6 flex flex-col">
          <Card role="status" aria-label="Appearance settings unavailable">
            <CardHeader>
              <div className="flex items-start justify-between gap-3">
                <CardTitle>Appearance</CardTitle>
                <Badge variant="muted" size="sm">
                  Unavailable
                </Badge>
              </div>
              <CardDescription>
                Appearance preferences are not part of the current app contract.
              </CardDescription>
            </CardHeader>
            <CardContent>
              <p className="text-sm text-muted">
                This section is informational only. No theme or UI scaling settings are saved yet.
              </p>
            </CardContent>
          </Card>

          <Card role="status" aria-label="Export defaults unavailable">
            <CardHeader>
              <div className="flex items-start justify-between gap-3">
                <CardTitle>Export Defaults</CardTitle>
                <Badge variant="muted" size="sm">
                  Unavailable
                </Badge>
              </div>
              <CardDescription>
                Export defaults are not part of the current app contract.
              </CardDescription>
            </CardHeader>
            <CardContent>
              <p className="text-sm text-muted">
                This section is informational only. No output directory, resolution, or format
                settings are saved yet.
              </p>
            </CardContent>
          </Card>
        </PageContent>
      </PageContainer>
    </Page>
  );
};
