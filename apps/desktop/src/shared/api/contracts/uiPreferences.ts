export interface StoredTheme {
  value: string;
  revision: number;
}
export interface ProjectPin {
  projectId: string;
  pinned: boolean;
  revision: number;
}
export interface ProjectPins {
  entries: ProjectPin[];
  migrated: boolean;
}
export interface LegacyPin {
  projectId: string;
  pinned: boolean;
}
