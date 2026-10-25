// Integration catalog for the mock (ids follow docs/INTEGRATIONS.md).

import type { Category, IntegrationInfo } from '../types.ts';

interface Entry {
  name: string;
  category: Category;
  description: string;
  kind?: 'pack' | 'adapter';
  status?: 'stable' | 'experimental';
  in_use_processes?: string[];
  version?: string;
}

export const CATALOG: Record<string, Entry> = {
  'node-modules': { name: 'node_modules', category: 'project-artifact', description: 'Installed npm, pnpm and yarn dependencies inside projects.', kind: 'pack' },
  'python-venv': { name: 'Python virtualenvs', category: 'project-artifact', description: 'Virtual environments inside projects.', kind: 'pack' },
  'js-build': { name: 'Build output', category: 'project-artifact', description: 'Framework build folders such as .next, dist and __pycache__.', kind: 'pack' },
  'rust-target': { name: 'Rust target', category: 'project-artifact', description: 'Cargo build output.', kind: 'pack', version: '1.91.0' },
  'npm-cache': { name: 'npm cache', category: 'package-cache', description: 'Downloaded npm packages.', kind: 'pack', version: '11.6.2' },
  'pnpm-store': { name: 'pnpm store', category: 'package-cache', description: 'The global pnpm content store.', kind: 'pack', version: '10.34.6' },
  'uv-cache': { name: 'uv cache', category: 'package-cache', description: 'Downloaded Python packages for uv.', kind: 'pack', version: '0.9.4' },
  'pip-cache': { name: 'pip cache', category: 'package-cache', description: 'Downloaded Python packages for pip.', kind: 'pack', version: '25.2' },
  'homebrew-cache': { name: 'Homebrew', category: 'package-cache', description: 'Old downloads and versions kept by Homebrew.', kind: 'pack', version: '4.6.18' },
  'gradle-cache': { name: 'Gradle caches', category: 'package-cache', description: 'Downloaded dependencies and build caches for Gradle.', kind: 'pack', version: '9.1.0' },
  playwright: { name: 'Playwright', category: 'package-cache', description: 'Browsers downloaded by Playwright.', kind: 'pack', version: '1.56.0' },
  docker: { name: 'Docker', category: 'container', description: 'Images, volumes, stopped containers and build cache.', kind: 'adapter', version: '28.5.1', in_use_processes: ['com.docker.backend'] },
  'xcode-derived-data': { name: 'Xcode DerivedData', category: 'mobile', description: 'Build products and indexes made by Xcode.', kind: 'pack', version: '26.1', in_use_processes: ['Xcode'] },
  'ios-device-support': { name: 'iOS DeviceSupport', category: 'mobile', description: 'Symbols Xcode copied from connected devices.', kind: 'pack', version: '26.1' },
  'android-avd': { name: 'Android emulators', category: 'mobile', description: 'Android virtual devices and their data.', kind: 'adapter', version: '36.1.9', in_use_processes: ['qemu-system-aarch64'] },
  'android-system-images': { name: 'Android system images', category: 'mobile', description: 'System images used to create emulators.', kind: 'adapter', version: '36.1.9' },
  ollama: { name: 'Ollama', category: 'model', description: 'Local models pulled with Ollama.', kind: 'adapter', version: '0.12.6', in_use_processes: ['ollama'] },
  huggingface: { name: 'Hugging Face', category: 'model', description: 'Models and datasets in the Hugging Face cache.', kind: 'pack' },
  codex: { name: 'Codex', category: 'ai-history', description: 'Codex CLI sessions, removed with codex delete.', kind: 'adapter', version: '0.48.0', in_use_processes: ['codex'] },
  'claude-code': { name: 'Claude Code', category: 'ai-history', description: 'Claude Code transcripts, moved whole.', kind: 'adapter', version: '2.4.1', in_use_processes: ['claude'] },
  cursor: { name: 'Cursor', category: 'ai-history', description: 'Cursor chat database and workspace storage.', kind: 'adapter', version: '1.9.2', in_use_processes: ['Cursor'] },
  antigravity: { name: 'Antigravity', category: 'ai-history', description: 'Antigravity conversation history.', kind: 'adapter', status: 'experimental', version: '1.2.0' },
};

export function integrationInfo(id: string): IntegrationInfo {
  const e = CATALOG[id];
  if (!e) throw new Error(`unknown integration ${id}`);
  return {
    id,
    name: e.name,
    category: e.category,
    description: e.description,
    kind: e.kind ?? 'pack',
    status: e.status ?? 'stable',
    in_use_processes: e.in_use_processes ?? [],
    os: [],
    docs_url: `https://github.com/Paras-ydv/kipple/blob/main/docs/integrations/${id}.md`,
  };
}
