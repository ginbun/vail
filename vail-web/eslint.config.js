import js from '@eslint/js';
import typescriptEslint from '@typescript-eslint/eslint-plugin';
import typescriptParser from '@typescript-eslint/parser';
import vueEslintParser from 'vue-eslint-parser';
import pluginVue from 'eslint-plugin-vue';
import pluginPrettier from 'eslint-plugin-prettier/recommended';
import pluginImportX from 'eslint-plugin-import-x';
import globals from 'globals';

export default [
  {
    ignores: ['node_modules', 'dist', 'web-dist', 'public'],
  },
  js.configs.recommended,
  ...pluginVue.configs['flat/recommended'],
  pluginPrettier,
  {
    files: ['**/*.{js,ts,vue,tsx,jsx}'],
    languageOptions: {
      ecmaVersion: 'latest',
      sourceType: 'module',
      globals: {
        ...globals.browser,
        ...globals.node,
      },
      parser: vueEslintParser,
      parserOptions: {
        parser: typescriptParser,
        extraFileExtensions: ['.vue'],
        ecmaFeatures: {
          jsx: true,
        },
      },
    },
    plugins: {
      '@typescript-eslint': typescriptEslint,
      'import-x': pluginImportX,
    },
    rules: {
      ...typescriptEslint.configs.recommended.rules,
      'no-undef': 'off',
      'prettier/prettier': 'warn',
      'vue/require-default-prop': 'off',
      'vue/singleline-html-element-content-newline': 'off',
      'vue/max-attributes-per-line': 'off',
      'vue/custom-event-name-casing': ['error', 'camelCase'],
      'vue/no-v-text': 'warn',
      'vue/padding-line-between-blocks': 'warn',
      'vue/require-direct-export': 'warn',
      'vue/multi-word-component-names': 'off',
      '@typescript-eslint/ban-ts-comment': 'off',
      '@typescript-eslint/no-unused-vars': 'warn',
      '@typescript-eslint/no-empty-function': 'warn',
      '@typescript-eslint/no-explicit-any': 'off',
      'no-debugger': process.env.NODE_ENV === 'production' ? 'error' : 'off',
      'no-param-reassign': 'off',
      'prefer-regex-literals': 'off',
      'import-x/no-extraneous-dependencies': 'off',
    },
  },
];
