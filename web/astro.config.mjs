// kipple.dev: a static site. No server runtime, no analytics, no external requests.
import svelte from '@astrojs/svelte';
import { defineConfig } from 'astro/config';

export default defineConfig({
  site: 'https://kipple.dev',
  output: 'static',
  integrations: [svelte()],
  build: {
    inlineStylesheets: 'auto',
  },
  devToolbar: {
    enabled: false,
  },
});
