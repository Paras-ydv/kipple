import { config } from '../packages/config/eslint.js';
import svelteConfig from './svelte.config.js';

// .astro files are checked by `astro check`; ESLint covers TypeScript and Svelte.
export default config({ svelteConfig });
