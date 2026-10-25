<script lang="ts">
  import ConfirmDialog from './components/ConfirmDialog.svelte';
  import ExplainSheet from './components/ExplainSheet.svelte';
  import Notice from './components/Notice.svelte';
  import SelectionSheet from './components/SelectionSheet.svelte';
  import ShareDialog from './components/ShareDialog.svelte';
  import Sidebar from './components/Sidebar.svelte';
  import Toasts from './components/Toasts.svelte';
  import { native } from './lib/client.ts';
  import { handleGlobalKey } from './lib/keyboard.ts';
  import { router } from './lib/router.svelte.ts';
  import { data } from './lib/stores/data.svelte.ts';
  import { planStore } from './lib/stores/plan.svelte.ts';
  import AiHistory from './routes/AiHistory.svelte';
  import Categories from './routes/Categories.svelte';
  import Overview from './routes/Overview.svelte';
  import Projects from './routes/Projects.svelte';
  import Quarantine from './routes/Quarantine.svelte';
  import Review from './routes/Review.svelte';
  import Settings from './routes/Settings.svelte';

  $effect(() => {
    void data.load();
  });

  // kipple picks the safe items once the first data arrives.
  $effect(() => {
    if (data.status === 'ready') planStore.ensure();
  });

  const titles: Record<string, string> = {
    overview: 'Overview',
    projects: 'Projects',
    ai: 'AI history',
    review: 'Review plan',
    quarantine: 'Quarantine',
    settings: 'Settings',
    categories: 'Categories',
  };

  $effect(() => {
    document.title = `${titles[router.route.name] ?? 'kipple'} – kipple`;
  });
</script>

<svelte:window onkeydown={handleGlobalKey} />

<div class="wall" class:native>
  <a class="k-skip-link" href="#main">Skip to content</a>
  <div class="win k-window">
    <Sidebar />
    <main id="main" tabindex="-1">
      {#if data.status === 'loading'}
        <div class="loading" role="status">Loading</div>
      {:else if data.status === 'unavailable'}
        <Notice
          title="This build can't talk to the kipple core yet."
          body="The app is ready, but the core API isn't wired in this build. The browser preview shows the app with sample data."
        />
      {:else if data.status === 'error'}
        <Notice title="kipple could not load your data." body={data.error ?? undefined} action={{ label: 'Try again', onclick: () => data.load() }} />
      {:else if router.route.name === 'overview'}
        <Overview />
      {:else if router.route.name === 'projects'}
        <Projects id={router.route.id} />
      {:else if router.route.name === 'ai'}
        <AiHistory tool={router.route.tool} />
      {:else if router.route.name === 'review'}
        <Review />
      {:else if router.route.name === 'quarantine'}
        <Quarantine />
      {:else if router.route.name === 'settings'}
        <Settings />
      {:else if router.route.name === 'categories'}
        <Categories id={router.route.id} />
      {/if}
    </main>
    <ExplainSheet />
    <SelectionSheet />
    <ConfirmDialog />
    <ShareDialog />
    <Toasts />
  </div>
</div>

<style>
  .wall {
    min-width: 100vw;
    min-height: 100vh;
    display: grid;
    place-items: center;
    padding: 16px;
    background: var(--k-wall);
    background-attachment: fixed;
    overflow: auto;
  }
  .win {
    position: relative;
    width: min(1328px, calc(100vw - 32px));
    height: min(820px, calc(100vh - 32px));
    min-width: 1100px;
    min-height: 720px;
    border-radius: var(--k-r-window);
    padding: 10px;
    display: flex;
    gap: 10px;
  }
  main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    padding: 6px 14px 14px 18px;
    gap: 12px;
    outline: none;
  }
  .loading {
    margin: auto;
    font-size: 13px;
    color: var(--k-text-4);
  }

  /* Inside Tauri the window itself is the glass; the desktop shows through. */
  .wall.native {
    padding: 0;
    background: transparent;
  }
  .wall.native .win {
    width: 100vw;
    height: 100vh;
    min-width: 0;
    min-height: 0;
    border: 0;
    border-radius: 0;
    box-shadow: none;
    background: rgba(15, 17, 29, 0.56);
  }
</style>
