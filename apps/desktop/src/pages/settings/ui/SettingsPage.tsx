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

export const SettingsPage = () => {
  return (
    <Page className="overflow-y-auto min-h-screen">
      <PageContainer size="md" className="py-12">
        <PageHeader className="mb-8">
          <PageHeaderGroup>
            <PageTitle>Settings</PageTitle>
            <PageDescription>Manage your app preferences and defaults.</PageDescription>
          </PageHeaderGroup>
        </PageHeader>

        <PageContent className="gap-6 flex flex-col">
          <Card>
            <CardHeader>
              <CardTitle>Appearance</CardTitle>
              <CardDescription>Customize the look and feel of Auralis.</CardDescription>
            </CardHeader>
            <CardContent>
              <p className="text-sm text-muted">Placeholder for theme settings, UI scaling, etc.</p>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>Export Defaults</CardTitle>
              <CardDescription>
                Configure default video formats and output locations.
              </CardDescription>
            </CardHeader>
            <CardContent>
              <p className="text-sm text-muted">
                Placeholder for export directory, resolution, and format.
              </p>
            </CardContent>
          </Card>
        </PageContent>
      </PageContainer>
    </Page>
  );
};
