module.exports = {
  root: true,
  parser: '@typescript-eslint/parser',
  plugins: ['boundaries'],
  extends: ['plugin:boundaries/recommended', 'plugin:storybook/recommended'],
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
    ]
  },
  rules: {
    'boundaries/dependencies': [
      2,
      {
        default: 'disallow',
        rules: [
          { from: 'app', allow: ['pages', 'widgets', 'features', 'entities', 'shared'] },
          { from: 'pages', allow: ['widgets', 'features', 'entities', 'shared'] },
          { from: 'widgets', allow: ['features', 'entities', 'shared'] },
          { from: 'features', allow: ['entities', 'shared'] },
          { from: 'entities', allow: ['shared'] },
          { from: 'shared', allow: [] }
        ]
      }
    ],
    'no-restricted-imports': [
      'error',
      {
        patterns: [
          {
            group: [
              '@/app/*/**',
              '@/pages/*/**',
              '@/widgets/*/**',
              '@/features/*/**',
              '@/entities/*/**'
            ],
            message: 'Direct access to internal slice files is forbidden. Import from the public index.ts API instead. Use relative paths for intra-slice imports.'
          }
        ]
      }
    ]
  }
};
