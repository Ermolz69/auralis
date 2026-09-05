import parser from '@typescript-eslint/parser';
import boundaries from 'eslint-plugin-boundaries';
import storybook from 'eslint-plugin-storybook';

export default [
  ...storybook.configs['flat/recommended'],
  {
    files: ['src/**/*.{ts,tsx}'],
    languageOptions: { parser },
    plugins: { boundaries },
    settings: {
      'import/resolver': {
        typescript: {
          alwaysTryTypes: true,
          project: './tsconfig.app.json',
        },
      },
      'boundaries/elements': [
        { type: 'app', pattern: 'src/app/**/*' },
        { type: 'pages', pattern: 'src/pages/**/*' },
        { type: 'widgets', pattern: 'src/widgets/**/*' },
        { type: 'features', pattern: 'src/features/**/*' },
        { type: 'entities', pattern: 'src/entities/**/*' },
        { type: 'shared', pattern: 'src/shared/**/*' },
      ],
    },
    rules: {
      ...boundaries.configs.recommended.rules,
      'boundaries/dependencies': [
        2,
        {
          default: 'disallow',
          rules: [
            {
              from: { type: 'app' },
              allow: [{ to: { type: ['pages', 'widgets', 'features', 'entities', 'shared'] } }],
            },
            {
              from: { type: 'pages' },
              allow: [{ to: { type: ['widgets', 'features', 'entities', 'shared'] } }],
            },
            {
              from: { type: 'widgets' },
              allow: [{ to: { type: ['features', 'entities', 'shared'] } }],
            },
            { from: { type: 'features' }, allow: [{ to: { type: ['entities', 'shared'] } }] },
            { from: { type: 'entities' }, allow: [{ to: { type: 'shared' } }] },
            { from: { type: 'shared' }, allow: [] },
          ],
        },
      ],
      'no-restricted-imports': [
        'error',
        {
          paths: [
            {
              name: 'lucide-react',
              message:
                'Use shared/ui/icon so icon sizing, colors, accessibility, and the approved registry stay consistent.',
            },
          ],
          patterns: [
            {
              group: [
                '@/app/*/**',
                '@/pages/*/**',
                '@/widgets/*/**',
                '@/features/*/**',
                '@/entities/*/**',
              ],
              message:
                'Direct access to internal slice files is forbidden. Import from the public index.ts API instead. Use relative paths for intra-slice imports.',
            },
          ],
        },
      ],
    },
  },
];
