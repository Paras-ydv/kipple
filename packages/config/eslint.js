// Shared flat ESLint config for TypeScript and Svelte 5 sources.
import js from '@eslint/js';
import svelte from 'eslint-plugin-svelte';
import globals from 'globals';
import ts from 'typescript-eslint';

/**
 * @param {{ svelteConfig?: object }} [options]
 */
export function config(options = {}) {
  return ts.config(
    {
      ignores: ['**/dist/**', '**/build/**', '**/.astro/**', '**/src-tauri/**', '**/generated/**'],
    },
    js.configs.recommended,
    ...ts.configs.recommended,
    ...svelte.configs.recommended,
    {
      languageOptions: {
        globals: { ...globals.browser, ...globals.node },
      },
    },
    {
      files: ['**/*.svelte', '**/*.svelte.ts', '**/*.svelte.js'],
      languageOptions: {
        parserOptions: {
          projectService: false,
          extraFileExtensions: ['.svelte'],
          parser: ts.parser,
          svelteConfig: options.svelteConfig,
        },
      },
    },
    {
      rules: {
        '@typescript-eslint/no-unused-vars': [
          'error',
          { argsIgnorePattern: '^_', varsIgnorePattern: '^_', destructuredArrayIgnorePattern: '^_' },
        ],
        // Svelte's {@html} is used only for SVG strings produced by our own renderers.
        'svelte/no-at-html-tags': 'off',
        // Maps and Sets here are local temporaries or replaced immutably, never mutated reactively.
        'svelte/prefer-svelte-reactivity': 'off',
        // Navigation is hash-based and handled by the app router.
        'svelte/no-navigation-without-resolve': 'off',
      },
    },
  );
}

export default config();
