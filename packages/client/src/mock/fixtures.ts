// Fixture data for MockClient. The numbers match the design boards
// (docs/design/boards): 184 GB of kipple, 76.5 GB safe, 107.5 GB to review,
// nomad-app 14.2 GB, Codex 21.4 GB in 318 sessions, and so on.
// Everything is fake: names, paths and session titles are made up.

import type {
  ActionKind,
  Category,
  Item,
  JournalEntry,
  LastUsed,
  Locator,
  ProjectRef,
  ReasonCode,
  Safety,
  SizePoint,
  VolumeInfo,
} from '../types.ts';
import { CATALOG } from './catalog.ts';
import { DAY, GB, MB, gb, hexId, iso, rng, split, ulid } from './util.ts';

export const HOME = '/Users/you';

export interface FixtureProject {
  ref: ProjectRef;
  lastDays: number;
  markers: string[];
  exists: boolean;
}

export interface Fixtures {
  now: number;
  items: Item[];
  projects: FixtureProject[];
  history: SizePoint[];
  journal: JournalEntry[];
  volume: VolumeInfo;
  scan: { id: string; startedAt: number; finishedAt: number; entries: number };
}

const native = (command_id: string): ActionKind => ({ type: 'native', command_id });
const QUARANTINE: ActionKind = { type: 'quarantine' };

interface Spec {
  integration: string;
  kind: string;
  title: string;
  bytes: number;
  safety: Safety;
  reasons: [ReasonCode, string][];
  actions: ActionKind[];
  project?: FixtureProject;
  lastDays?: number;
  locator: Locator;
  details?: Record<string, unknown>;
  restore_hint?: string;
  notes?: string[];
}

export function buildFixtures(now = Date.now()): Fixtures {
  const rand = rng(0x6b1991e);
  const items: Item[] = [];
  const projects: FixtureProject[] = [];
  let seq = 0;

  const ago = (days: number) => now - days * DAY;

  function project(name: string, root: string, lastDays: number, markers: string[], exists = true): FixtureProject {
    const p: FixtureProject = {
      ref: { id: hexId(`project:${root}`), name, root: `${HOME}/${root}` },
      lastDays,
      markers,
      exists,
    };
    projects.push(p);
    return p;
  }

  function add(s: Spec): Item {
    const id = hexId(`${s.integration}\0${s.title}\0${seq++}`);
    const lastUsed: LastUsed | null =
      s.lastDays !== undefined
        ? { at: iso(ago(s.lastDays)), source: s.project ? 'git_commit' : 'item_mtime' }
        : s.project
          ? { at: iso(ago(s.project.lastDays)), source: 'git_commit' }
          : null;
    const category = CATALOG[s.integration]?.category as Category;
    const item: Item = {
      id,
      integration: s.integration,
      category,
      kind: s.kind,
      title: s.title,
      locator: s.locator,
      size: {
        apparent: Math.round(s.bytes * 1.04),
        on_disk: s.bytes,
        reclaimable: s.bytes,
        file_count: s.locator.type === 'path' ? Math.round(s.bytes / 48_000) : null,
      },
      project: s.project ? { ...s.project.ref } : null,
      last_used: lastUsed,
      safety: s.safety,
      reasons: s.reasons.map(([code, text]) => ({ code, text })),
      actions: s.safety === 'keep' ? [] : s.actions,
      details: s.details ?? {},
      notes: s.notes ?? [],
      restore_hint: s.restore_hint ?? null,
      identity: s.locator.type === 'path' ? { type: 'inode', dev: 16777232, ino: 1000000 + seq } : null,
    };
    items.push(item);
    return item;
  }

  const path = (p: string): Locator => ({ type: 'path', path: p.startsWith('/') ? p : `${HOME}/${p}` });
  const docker = (kind: string, seed: string): Locator => ({ type: 'docker', kind, id: hexId(seed).slice(0, 12) });

  // ---- Projects ------------------------------------------------------------
  const nomad = project('nomad-app', 'work/nomad-app', 214, ['package.json', '.git', 'docker-compose.yml', 'pyproject.toml']);
  const llm = project('llm-bench', 'play/llm-bench', 61, ['pyproject.toml', '.git', 'package.json']);
  const shop = project('android-shop', 'work/android-shop', 152, ['settings.gradle.kts', '.git', 'package.json', 'docker-compose.yml']);
  const kipple = project('kipple', 'code/kipple', 0.1, ['Cargo.toml', '.git', 'package.json']);
  const portal = project('client-portal', 'work/client-portal', 22, ['package.json', '.git', 'docker-compose.yml']);
  const portfolio = project('portfolio-v3', 'play/portfolio-v3', 372, ['package.json', '.git']);
  const expense = project('rn-expense', 'play/rn-expense', 276, ['package.json', '.git', 'app.json']);

  const fillerNames = [
    'go-ingest', 'dotfiles', 'blog', 'invoice-api', 'design-tokens', 'chrome-ext', 'astro-docs', 'ml-notebooks',
    'weather-cli', 'tauri-notes', 'shop-admin', 'pricing-page', 'slack-bot', 'graphql-gateway', 'mobile-onboarding',
    'pdf-tools', 'auth-service', 'landing-2024', 'data-pipeline', 'k8s-sandbox', 'rust-playground', 'swift-widgets',
    'flutter-demo', 'django-crm', 'rails-legacy', 'nextjs-starter', 'vite-lab', 'svelte-kit-app', 'expo-camera',
    'terraform-infra', 'analytics-etl', 'cli-template', 'image-resizer', 'markdown-wiki', 'budget-tracker',
    'video-transcoder', 'queue-worker', 'feature-flags', 'email-templates', 'search-service', 'pwa-offline',
    'game-of-life', 'chat-widget', 'scheduler', 'notes-sync', 'ios-habits', 'api-mocks', 'storybook-ui',
  ];
  const deletedNames = ['old-scraper', 'hackathon-2025', 'tmp-vite-test', 'demo-call', 'exp-agents', 'interview-task'];
  const filler = fillerNames.map((n, i) =>
    project(n, `${i % 3 === 0 ? 'work' : i % 3 === 1 ? 'play' : 'code'}/${n}`, Math.round(3 + rand() * 420), ['.git', 'package.json']),
  );
  const deleted = deletedNames.map((n) => project(n, `play/${n}`, Math.round(200 + rand() * 300), ['.git'], false));
  const goIngest = filler[0] as FixtureProject;
  goIngest.markers = ['go.mod', '.git'];
  goIngest.lastDays = 58;
  const oldScraper = deleted[0] as FixtureProject;
  oldScraper.lastDays = 280;

  // Filler attribution: least-loaded project first. go-ingest holds one big Codex
  // session; three iOS projects hold DerivedData and archives.
  const reserved = new Set(['go-ingest', 'swift-widgets', 'flutter-demo', 'ios-habits']);
  const general = filler.filter((p) => !reserved.has(p.ref.name));
  const byName = (n: string) => filler.find((p) => p.ref.name === n) as FixtureProject;
  const load = new Map<FixtureProject, number>();
  const pickFiller = (bytes: number, pool: FixtureProject[] = general): FixtureProject => {
    let best = pool[0] as FixtureProject;
    for (const p of pool) if ((load.get(p) ?? 0) < (load.get(best) ?? 0)) best = p;
    load.set(best, (load.get(best) ?? 0) + bytes);
    return best;
  };

  // ---- Reason texts ----------------------------------------------------------
  const R = {
    npmInstall: ['REGENERABLE', 'npm install brings it back.'] as [ReasonCode, string],
    rebuild: ['REGENERABLE', 'The next build makes it again.'] as [ReasonCode, string],
    venv: ['CONTAINS_USER_DATA', 'Packages installed by hand are not listed anywhere and would be lost.'] as [ReasonCode, string],
    recent: ['RECENTLY_USED', 'You built this project today.'] as [ReasonCode, string],
    redownload: (tool: string) => ['REDOWNLOADABLE', `${tool} downloads it again when needed.`] as [ReasonCode, string],
    chat: ['CHAT_HISTORY', 'Chat history you may want to read again.'] as [ReasonCode, string],
  };

  // ---- Containers: 61.2 GB, 22.1 GB safe --------------------------------------
  add({
    integration: 'docker', kind: 'docker_build_cache', title: 'Docker build cache', bytes: gb(14.3), safety: 'safe',
    reasons: [['REGENERABLE', 'Docker rebuilds these layers when a build needs them.']],
    actions: [native('docker.builder.prune')], locator: docker('docker_build_cache', 'buildkit'), lastDays: 1,
    restore_hint: 'docker build',
  });
  const dangling = split(gb(4.4), 14, rand);
  const danglingOwners: [FixtureProject, number][] = [
    [nomad, gb(1.2)], [nomad, gb(0.9)], [nomad, gb(0.7)], [portal, gb(0.35)], [portal, gb(0.25)],
    ...dangling.map((b) => [pickFiller(b), b] as [FixtureProject, number]),
  ];
  danglingOwners.forEach(([p, b], i) =>
    add({
      integration: 'docker', kind: 'docker_image', title: `${p.ref.name}-${['web', 'api', 'worker'][i % 3]} image (dangling)`,
      bytes: b, safety: 'safe', reasons: [['REGENERABLE', 'No tag and no container uses it.']],
      actions: [native('docker.image.remove')], locator: docker('image', `img${i}`), project: p,
      details: { dangling: true }, lastDays: 30 + i * 9,
    }),
  );
  add({
    integration: 'docker', kind: 'docker_volume', title: 'volume nomad_pgdata', bytes: gb(6.1), safety: 'review',
    reasons: [
      ['CONTAINS_USER_DATA', 'Docker volume from the nomad-app compose project. No container mounts it.'],
      ['CONTAINS_USER_DATA', 'It holds Postgres data, so removing it deletes those database rows for good.'],
    ],
    actions: [native('docker.volume.remove')], locator: docker('volume', 'nomad_pgdata'), project: nomad,
    details: { compose_project: 'nomad-app', mounted: false },
  });
  add({
    integration: 'docker', kind: 'docker_volume', title: 'volume shop_redis', bytes: gb(2.3), safety: 'review',
    reasons: [['CONTAINS_USER_DATA', 'Redis data from the android-shop compose project. No container mounts it.']],
    actions: [native('docker.volume.remove')], locator: docker('volume', 'shop_redis'), project: shop,
  });
  add({
    integration: 'docker', kind: 'docker_image', title: 'image postgres:16 (unused)', bytes: gb(0.4), safety: 'review',
    reasons: [['LARGE_REDOWNLOAD', 'No container uses it. Pulling it again takes a while.']],
    actions: [native('docker.image.remove')], locator: docker('image', 'postgres16'), project: shop,
  });
  const stopped = [...split(gb(0.6), 4, rand).map((b) => [shop, b] as const), ...split(gb(0.6), 3, rand).map((b) => [pickFiller(b), b] as const)];
  stopped.forEach(([p, b], i) =>
    add({
      integration: 'docker', kind: 'docker_container', title: `stopped container ${p.ref.name}-${i + 1}`, bytes: b,
      safety: 'review', reasons: [['CONTAINS_USER_DATA', 'Stopped container. Files written inside it are lost when it is removed.']],
      actions: [native('docker.container.remove')], locator: docker('container', `ctr${i}`), project: p, lastDays: 40 + i * 11,
    }),
  );
  // Remaining review volumes (29.1 GB) spread over other projects.
  split(gb(29.1), 24, rand, 1.2).forEach((b, i) => {
    const p = pickFiller(b);
    add({
      integration: 'docker', kind: 'docker_volume', title: `volume ${p.ref.name.replace(/-/g, '_')}_${['db', 'data', 'cache'][i % 3]}`,
      bytes: b, safety: 'review', reasons: [['CONTAINS_USER_DATA', 'No container mounts it, but it may hold data you need.']],
      actions: [native('docker.volume.remove')], locator: docker('volume', `vol${i}`), project: p,
    });
  });

  // ---- Package caches: 27.5 GB, all safe ------------------------------------
  const cache = (integration: string, kind: string, title: string, gbs: number, p: string, action: ActionKind, tool: string, hint?: string) =>
    add({
      integration, kind, title, bytes: gb(gbs), safety: 'safe', reasons: [R.redownload(tool)], actions: [action],
      locator: path(p), lastDays: 1 + Math.round(rand() * 6), restore_hint: hint,
    });
  cache('npm-cache', 'npm_cache', 'npm cache', 8.1, '.npm/_cacache', native('npm.cache.clean'), 'npm', 'npm install');
  cache('npm-cache', 'npx_cache', 'npx cache', 0.5, '.npm/_npx', native('npm.cache.clean'), 'npx');
  cache('uv-cache', 'uv_cache', 'uv cache', 4.1, 'Library/Caches/uv', native('uv.cache.clean'), 'uv');
  cache('pip-cache', 'pip_cache', 'pip cache', 2.2, 'Library/Caches/pip', native('pip.cache.purge'), 'pip');
  cache('homebrew-cache', 'brew_downloads', 'Homebrew downloads', 2.4, 'Library/Caches/Homebrew/downloads', native('brew.cleanup'), 'Homebrew');
  cache('homebrew-cache', 'brew_old_versions', 'Old Homebrew versions', 1.0, '/opt/homebrew/Cellar', native('brew.cleanup'), 'Homebrew');
  cache('pnpm-store', 'pnpm_store', 'pnpm store, unused packages', 1.8, 'Library/pnpm/store/v10', native('pnpm.store.prune'), 'pnpm');
  cache('pnpm-store', 'pnpm_store', 'pnpm store, old index', 0.8, 'Library/pnpm/store/v3', native('pnpm.store.prune'), 'pnpm');
  cache('gradle-cache', 'gradle_cache', 'Gradle module cache', 2.0, '.gradle/caches/modules-2', QUARANTINE, 'Gradle');
  cache('gradle-cache', 'gradle_cache', 'Gradle build cache', 1.4, '.gradle/caches/build-cache-1', QUARANTINE, 'Gradle');
  cache('gradle-cache', 'gradle_cache', 'Gradle wrapper dists', 0.7, '.gradle/wrapper/dists', QUARANTINE, 'Gradle');
  cache('playwright', 'playwright_browser', 'Playwright Chromium 1187', 1.1, 'Library/Caches/ms-playwright/chromium-1187', QUARANTINE, 'Playwright');
  cache('playwright', 'playwright_browser', 'Playwright WebKit 2215', 0.9, 'Library/Caches/ms-playwright/webkit-2215', QUARANTINE, 'Playwright');
  cache('playwright', 'playwright_browser', 'Playwright Firefox 1490', 0.5, 'Library/Caches/ms-playwright/firefox-1490', QUARANTINE, 'Playwright');

  // ---- Project artifacts: 24.0 GB, 18.7 GB safe -------------------------------
  const nodeModules: [FixtureProject, number][] = [
    [nomad, gb(1.9)], [llm, gb(0.8)], [shop, gb(1.8)], [kipple, gb(0.4)], [portal, gb(2.9)], [portfolio, gb(2.6)], [expense, gb(1.8)],
  ];
  // One node_modules per project: 41 projects in all.
  split(gb(2.7), 34, rand).forEach((b, i) => {
    const p = general[i] as FixtureProject;
    load.set(p, (load.get(p) ?? 0) + b);
    nodeModules.push([p, b]);
  });
  nodeModules.forEach(([p, b]) =>
    add({
      integration: 'node-modules', kind: 'node_modules', title: 'node_modules', bytes: b, safety: 'safe',
      reasons: [R.npmInstall], actions: [QUARANTINE], locator: path(`${p.ref.root.slice(HOME.length + 1)}/node_modules`),
      project: p, restore_hint: 'npm install',
    }),
  );
  const builds: [FixtureProject, string, string, number][] = [
    [nomad, 'nextjs', 'apps/web/.next', gb(1.2)], [nomad, 'pycache', 'Python caches', gb(0.2)],
    [shop, 'gradle_build', 'app/build', gb(0.5)], [portal, 'nextjs', '.next', gb(0.6)],
    [portfolio, 'dist', 'dist', gb(0.4)], [expense, 'expo', '.expo', gb(0.2)],
  ];
  split(gb(0.7), 29, rand).forEach((b, i) => builds.push([pickFiller(b), 'dist', ['dist', '.next', 'build', '.turbo'][i % 4] as string, b]));
  builds.forEach(([p, kind, title, b]) =>
    add({
      integration: 'js-build', kind, title, bytes: b, safety: 'safe', reasons: [R.rebuild], actions: [QUARANTINE],
      locator: path(`${p.ref.root.slice(HOME.length + 1)}/${kind === 'pycache' ? '**/__pycache__' : title}`), project: p,
    }),
  );
  add({
    integration: 'rust-target', kind: 'rust_target', title: 'target', bytes: gb(1.9), safety: 'review', reasons: [R.recent],
    actions: [QUARANTINE], locator: path('code/kipple/target'), project: kipple, lastDays: 0.05, restore_hint: 'cargo build',
  });
  const venvs: [FixtureProject, number][] = [[llm, gb(2.0)], [shop, gb(0.6)]];
  split(gb(0.8), 17, rand).forEach((b) => venvs.push([pickFiller(b), b]));
  venvs.forEach(([p, b]) =>
    add({
      integration: 'python-venv', kind: 'python_venv', title: '.venv', bytes: b, safety: 'review', reasons: [R.venv],
      actions: [QUARANTINE], locator: path(`${p.ref.root.slice(HOME.length + 1)}/.venv`), project: p,
      restore_hint: 'uv sync',
    }),
  );

  // ---- Mobile: 21.6 GB, 8.2 GB safe -------------------------------------------
  const dd: [number, FixtureProject][] = [[gb(3.3), byName('swift-widgets')], [gb(2.7), byName('flutter-demo')]];
  dd.forEach(([b, p], i) =>
    add({
      integration: 'xcode-derived-data', kind: 'xcode_derived_data', title: `DerivedData ${p.ref.name}`, bytes: b, safety: 'safe',
      reasons: [['REGENERABLE', 'Xcode rebuilds it on the next build.']], actions: [QUARANTINE],
      locator: path(`Library/Developer/Xcode/DerivedData/${p.ref.name}-${hexId(p.ref.name).slice(0, 8)}`), project: p,
      lastDays: 20 + i * 30,
    }),
  );
  add({
    integration: 'android-system-images', kind: 'android_system_image', title: 'Android 10 system image', bytes: gb(2.2), safety: 'safe',
    reasons: [['UNUSED_BY_ANY_EMULATOR', 'No emulator uses it.']], actions: [QUARANTINE],
    locator: path('Library/Android/sdk/system-images/android-29'), lastDays: 400,
  });
  const avd = (name: string, gbs: number, days: number) =>
    add({
      integration: 'android-avd', kind: 'android_avd', title: `Emulator ${name}`, bytes: gb(gbs), safety: 'review',
      reasons: [['CONTAINS_USER_DATA', 'An emulator with its apps and data. Removing it deletes them.']],
      actions: [QUARANTINE], locator: path(`.android/avd/${name}.avd`), lastDays: days,
    });
  avd('Pixel_7_API_34', 3.06, 152);
  avd('Pixel_5_API_30', 3.1, 330);
  avd('Tablet_API_33', 2.0, 410);
  ['iPhone16,1 18.6', 'iPhone15,2 17.5', 'iPad13,4 17.1'].forEach((name, i) =>
    add({
      integration: 'ios-device-support', kind: 'device_support', title: `DeviceSupport ${name}`, bytes: gb([1.4, 1.0, 0.7][i] ?? 1),
      safety: 'review', reasons: [['LARGE_REDOWNLOAD', 'Xcode copies it again from the device the next time you connect it.']],
      actions: [QUARANTINE], locator: path(`Library/Developer/Xcode/iOS DeviceSupport/${name}`), lastDays: 90 + i * 120,
    }),
  );
  // Archives attributed to a project; with Antigravity copies this makes "61 projects holding 112 GB".
  const archiveOwner = byName('ios-habits');
  add({
    integration: 'xcode-derived-data', kind: 'xcode_archive', title: 'Xcode archives', bytes: gb(2.14), safety: 'review',
    reasons: [['CONTAINS_USER_DATA', 'Archives you may need to symbolicate old crash reports.']], actions: [QUARANTINE],
    locator: path('Library/Developer/Xcode/Archives/ios-habits'), project: archiveOwner, lastDays: 200,
  });

  // ---- Models: 10.8 GB, all review ---------------------------------------------
  const model = (integration: string, kind: string, title: string, gbs: number, p: string, action: ActionKind, proj?: FixtureProject) =>
    add({
      integration, kind, title, bytes: gb(gbs), safety: 'review',
      reasons: [['LARGE_REDOWNLOAD', `A ${gbs.toFixed(1)} GB download to get back.`]], actions: [action],
      locator: path(p), project: proj, lastDays: 15 + Math.round(rand() * 90),
    });
  model('huggingface', 'model_dir', 'Local model weights', 3.7, 'play/llm-bench/models', QUARANTINE, llm);
  model('ollama', 'ollama_model', 'llama3.1:8b', 4.9, '.ollama/models/llama3.1-8b', native('ollama.rm'));
  model('ollama', 'ollama_model', 'qwen2.5-coder:1.5b', 1.0, '.ollama/models/qwen2.5-coder-1.5b', native('ollama.rm'));
  model('huggingface', 'hf_model', 'sentence-transformers/all-mpnet-base-v2', 0.6, '.cache/huggingface/hub/all-mpnet', QUARANTINE);
  model('huggingface', 'hf_model', 'openai/whisper-small', 0.4, '.cache/huggingface/hub/whisper-small', QUARANTINE);
  model('huggingface', 'hf_dataset', 'squad dataset', 0.2, '.cache/huggingface/datasets/squad', QUARANTINE);

  // ---- AI history: 38.9 GB --------------------------------------------------
  const prompts = [
    'Add pagination to the orders endpoint', 'Why does the build fail on CI', 'Write tests for the price parser',
    'Rename the user service to accounts', 'Speed up the image upload', 'Fix the dark mode toggle',
    'Move config to environment variables', 'Explain this migration error', 'Add a retry to the webhook sender',
    'Clean up unused imports', 'Make the table sortable', 'Upgrade to React 19', 'Find the memory leak in the worker',
    'Add OAuth login with GitHub', 'Convert the scripts to TypeScript', 'Debug the flaky date test',
    'Generate a changelog from commits', 'Split the monolith router', 'Add caching to the search API',
    'Document the deploy steps', 'Fix the Android back button', 'Set up the GitHub Actions deploy',
    'Write a seed script for Postgres', 'Add rate limiting', 'Make the CLI output JSON', 'Profile the slow page',
  ];
  let promptIdx = 0;
  const nextPrompt = () => prompts[promptIdx++ % prompts.length] as string;

  interface Session {
    title?: string;
    bytes: number;
    project: FixtureProject;
    days?: number;
    flag?: 'tool' | 'secret' | 'orphan' | 'open';
    suggested?: boolean;
    secretKind?: string;
  }
  const codexSessions: Session[] = [
    { title: 'Refactor the ingest workers to use a queue', bytes: gb(2.9), project: goIngest, days: 58, flag: 'tool', suggested: true },
    { title: 'Fix flaky auth tests in CI', bytes: gb(1.1), project: nomad, days: 214, flag: 'secret', suggested: true, secretKind: 'OpenAI API key' },
    { title: 'Port the scraper to Playwright', bytes: 860 * MB, project: oldScraper, days: 280, flag: 'orphan', suggested: true },
    { title: 'Add offline mode to the expense app', bytes: 640 * MB, project: expense, days: 276, suggested: true },
    { title: 'Explain this stack trace', bytes: 38 * MB, project: kipple, days: 4 / 1440, flag: 'open' },
  ];
  // 19 more suggested sessions (5.9 GB): old, large, in projects you left.
  const suggested: [FixtureProject, number][] = [
    [llm, 0.6], [llm, 0.55], [llm, 0.5], [llm, 0.45], [llm, 0.42],
    [shop, 0.4], [shop, 0.38], [shop, 0.35], [shop, 0.33],
    [portfolio, 0.3], [portfolio, 0.28], [portfolio, 0.26],
    [portal, 0.24], [portal, 0.22], [portal, 0.2],
  ];
  suggested.forEach(([p, g], i) =>
    codexSessions.push({
      bytes: gb(g), project: p, days: Math.max(45, p.lastDays + 5 + i * 3), suggested: true,
      flag: i === 0 || i === 6 ? 'secret' : g > 0.5 ? 'tool' : undefined,
      secretKind: i === 0 || i === 6 ? 'GitHub token' : undefined,
    }),
  );
  [0.15, 0.11, 0.09, 0.07].forEach((g, i) => {
    const p = filler[1 + i * 7] as FixtureProject;
    codexSessions.push({ bytes: gb(g), project: p, days: 120 + i * 40, suggested: true });
  });
  // The rest of each project's Codex total.
  const rest: [FixtureProject, number, number][] = [
    [nomad, 13, gb(0.3)], [llm, 35, gb(3.2 - 2.52)], [shop, 21, gb(2.2 - 1.46)], [kipple, 29, gb(3.1) - 38 * MB],
    [portal, 19, gb(1.7 - 0.66)], [portfolio, 8, gb(0.9 - 0.84)], [expense, 8, gb(1.4) - 640 * MB],
  ];
  rest.forEach(([p, n, total]) =>
    split(total, n, rand).forEach((b) => codexSessions.push({ bytes: b, project: p, days: p.lastDays + rand() * 40 })),
  );
  // Remaining 161 sessions on other projects (7.5 GB total for others).
  const otherCodex = gb(7.5) - gb(2.9) - 860 * MB - gb(0.42);
  split(otherCodex, 161, rand).forEach((b, i) => {
    const p = filler[1 + (i % (filler.length - 1))] as FixtureProject;
    codexSessions.push({ bytes: b, project: p, days: 1 + rand() * 330 });
  });

  codexSessions.forEach((s, i) => {
    const title = s.title ?? nextPrompt();
    const open = s.flag === 'open';
    const reasons: [ReasonCode, string][] = open
      ? [['TOOL_RUNNING', 'This session is open in Codex right now.']]
      : [R.chat];
    if (s.flag === 'secret') reasons.push(['MAY_CONTAIN_SECRETS', `A pasted ${s.secretKind ?? 'secret'} is inside. Rotate it before you remove the session.`]);
    if (s.flag === 'orphan') reasons.push(['ORPHANED_PROJECT', 'Its project folder was deleted.']);
    if (s.flag === 'tool') reasons.push(['CHAT_HISTORY', 'Mostly tool output, such as file dumps and test logs.']);
    add({
      integration: 'codex', kind: 'codex_session', title, bytes: s.bytes, safety: open ? 'keep' : 'review', reasons,
      actions: [native('codex.delete')], locator: { type: 'logical', store: 'codex', key: hexId(`codex${i}`) },
      project: s.project, lastDays: s.days ?? 30,
      details: {
        first_prompt: title,
        flag: s.flag ?? null,
        secret: s.secretKind ?? null,
        suggested: !!s.suggested,
        open,
        turns: Math.max(3, Math.round(s.bytes / (12 * MB))),
      },
    });
  });

  // Claude Code: 80 sessions, 3.2 GB; 41 in deleted folders.
  const claude: [FixtureProject, number][] = [];
  split(gb(0.6), 9, rand).forEach((b) => claude.push([nomad, b]));
  split(gb(0.7), 15, rand).forEach((b) => claude.push([kipple, b]));
  split(gb(1.2), 41, rand).forEach((b, i) => claude.push([deleted[i % deleted.length] as FixtureProject, b]));
  split(gb(0.7), 15, rand).forEach((b) => claude.push([pickFiller(b), b]));
  claude.forEach(([p, b], i) => {
    const title = nextPrompt();
    const orphan = !p.exists;
    add({
      integration: 'claude-code', kind: 'claude_code_session', title, bytes: b, safety: 'review',
      reasons: orphan ? [R.chat, ['ORPHANED_PROJECT', 'Its project folder was deleted.']] : [R.chat],
      actions: [QUARANTINE], locator: path(`.claude/projects/-Users-you-${p.ref.root.split('/').slice(-2).join('-')}/${hexId(`cl${i}`).slice(0, 8)}.jsonl`),
      project: p, lastDays: orphan ? 31 + rand() * 25 : p.lastDays + rand() * 25,
      details: { first_prompt: title, flag: orphan ? 'orphan' : null },
    });
  });

  // Cursor: 11.8 GB.
  add({
    integration: 'cursor', kind: 'cursor_state_db', title: 'Cursor chat database', bytes: gb(9.6), safety: 'review',
    reasons: [['CHAT_HISTORY', 'Chat data that only Cursor can remove safely. Use Delete Old Chats in Cursor.'], ['FREE_PAGES', '1.9 GB of the file is free pages that compacting gives back.']],
    actions: [{ type: 'compact', method: 'sqlite_vacuum' }], locator: path('Library/Application Support/Cursor/User/globalStorage/state.vscdb'),
    lastDays: 0.2, details: { free_pages_bytes: gb(1.9) },
  });
  split(gb(2.2), 5, rand).forEach((b, i) => {
    const p = deleted[(i + 1) % deleted.length] as FixtureProject;
    add({
      integration: 'cursor', kind: 'cursor_workspace_storage', title: `Workspace storage for ${p.ref.name}`, bytes: b, safety: 'review',
      reasons: [['ORPHANED_PROJECT', 'Its project folder was deleted.']], actions: [QUARANTINE],
      locator: path(`Library/Application Support/Cursor/User/workspaceStorage/${hexId(`ws${i}`)}`), project: p,
      details: { flag: 'orphan' },
    });
  });

  // Antigravity: 2.5 GB of migrated copies (experimental).
  split(gb(2.5), 8, rand).forEach((b, i) =>
    add({
      project: pickFiller(b),
      integration: 'antigravity', kind: 'antigravity_conversation', title: `Old copy of conversation ${i + 1}`, bytes: b, safety: 'review',
      reasons: [['MIGRATED_COPY', 'An old copy Antigravity kept after migrating the conversation.'], ['EXPERIMENTAL', 'Antigravity support is experimental.']],
      actions: [QUARANTINE], locator: path(`.gemini/antigravity/conversations.bak/${hexId(`ag${i}`).slice(0, 12)}`),
      lastDays: 60 + i * 5,
    }),
  );

  // ---- History (SizePoints) ----------------------------------------------------
  const history: SizePoint[] = [];
  const monthly = [0.2, 0.3, 0.5, 0.6, 0.9, 1.1, 1.4, 1.8, 2.4, 3.1, 3.9, 5.2];
  const nowDate = new Date(now);
  let cum = 0;
  history.push({ scope: 'integration', key: 'codex', ts: iso(Date.UTC(nowDate.getUTCFullYear(), nowDate.getUTCMonth() - 12, 1)), reclaimable: 0 });
  monthly.forEach((m, i) => {
    cum += gb(m);
    const end = i === 11 ? now : Date.UTC(nowDate.getUTCFullYear(), nowDate.getUTCMonth() - 11 + i + 1, 1) - 1;
    history.push({ scope: 'integration', key: 'codex', ts: iso(end), reclaimable: Math.min(cum, gb(21.4)) });
  });
  const weekAgo = iso(now - 7 * DAY);
  const growth: [string, number, number][] = [
    ['codex', gb(21.4), gb(6.2)],
    ['docker/docker_build_cache', gb(14.3), gb(3.1)],
    ['cursor/chat_db', gb(9.6), gb(1.4)],
  ];
  growth.forEach(([key, nowBytes, delta]) => {
    history.push({ scope: 'integration', key, ts: weekAgo, reclaimable: nowBytes - delta });
    history.push({ scope: 'integration', key, ts: iso(now), reclaimable: nowBytes });
  });
  history.push({ scope: 'total', key: 'total', ts: weekAgo, reclaimable: gb(184 - 10.7) });
  history.push({ scope: 'total', key: 'total', ts: iso(now), reclaimable: gb(184) });

  // ---- Journal: an earlier run, already purged ----------------------------------
  const earlierPlan = ulid(now - 12 * DAY, 'earlier-plan');
  const journal: JournalEntry[] = [
    ['npm cache', 'npm-cache', native('npm.cache.clean'), gb(5.4)],
    ['Docker build cache', 'docker', native('docker.builder.prune'), gb(9.8)],
    ['node_modules in 12 projects', 'node-modules', QUARANTINE, gb(3.1)],
    ['Playwright Chromium 1169', 'playwright', QUARANTINE, gb(1.0)],
  ].map(([title, integration, action, freed], i) => ({
    ts: iso(now - 12 * DAY + i * 4000),
    plan_id: earlierPlan,
    item_id: hexId(`old${i}`),
    integration: integration as string,
    title: title as string,
    action: action as ActionKind,
    outcome: { status: 'done', freed: freed as number },
    freed: freed as number,
    original_path: (action as ActionKind).type === 'quarantine' ? `${HOME}/old/${i}` : null,
    quarantine_id: null,
  }));

  return {
    now,
    items,
    projects,
    history,
    journal: journal.reverse(),
    volume: { name: 'Macintosh HD', mount: '/', capacity: 512 * GB, free: 41 * GB, home: '/Users/you' },
    scan: { id: ulid(now - 2 * 60_000, 'scan'), startedAt: now - 2 * 60_000 - 18_000, finishedAt: now - 2 * 60_000, entries: 1_020_000 },
  };
}
