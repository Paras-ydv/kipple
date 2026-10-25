// Hash router. Hash URLs work the same in the browser preview and in Tauri.

export type Route =
  | { name: 'overview' }
  | { name: 'projects'; id?: string }
  | { name: 'ai'; tool?: string }
  | { name: 'review' }
  | { name: 'quarantine' }
  | { name: 'settings' }
  | { name: 'categories'; id?: string };

export type RouteName = Route['name'];

/** Sidebar order; ⌘1–⌘6 follow it. */
export const NAV: { name: RouteName; label: string; href: string; icon: 'overview' | 'projects' | 'ai' | 'plan' | 'quarantine' | 'settings' }[] = [
  { name: 'overview', label: 'Overview', href: '#/', icon: 'overview' },
  { name: 'projects', label: 'Projects', href: '#/projects', icon: 'projects' },
  { name: 'ai', label: 'AI history', href: '#/ai', icon: 'ai' },
  { name: 'review', label: 'Review plan', href: '#/review', icon: 'plan' },
  { name: 'quarantine', label: 'Quarantine', href: '#/quarantine', icon: 'quarantine' },
  { name: 'settings', label: 'Settings', href: '#/settings', icon: 'settings' },
];

export function parse(hash: string): Route {
  const parts = hash.replace(/^#\/?/, '').split('/').filter(Boolean).map(decodeURIComponent);
  const [head, arg] = parts;
  switch (head) {
    case undefined:
    case '':
    case 'overview':
      return { name: 'overview' };
    case 'projects':
      return { name: 'projects', id: arg };
    case 'ai':
      return { name: 'ai', tool: arg };
    case 'review':
      return { name: 'review' };
    case 'quarantine':
      return { name: 'quarantine' };
    case 'settings':
      return { name: 'settings' };
    case 'categories':
      return { name: 'categories', id: arg };
    default:
      return { name: 'overview' };
  }
}

export function href(route: Route): string {
  switch (route.name) {
    case 'overview':
      return '#/';
    case 'projects':
      return route.id ? `#/projects/${encodeURIComponent(route.id)}` : '#/projects';
    case 'ai':
      return route.tool ? `#/ai/${encodeURIComponent(route.tool)}` : '#/ai';
    case 'categories':
      return route.id ? `#/categories/${encodeURIComponent(route.id)}` : '#/categories';
    default:
      return `#/${route.name}`;
  }
}

class Router {
  route: Route = $state(parse(typeof location === 'undefined' ? '' : location.hash));

  constructor() {
    if (typeof window !== 'undefined') {
      window.addEventListener('hashchange', () => {
        this.route = parse(location.hash);
      });
    }
  }

  go(route: Route, opts: { replace?: boolean } = {}) {
    const next = href(route);
    if (opts.replace) {
      history.replaceState(null, '', next);
      this.route = parse(next);
    } else if (location.hash !== next) {
      location.hash = next;
    } else {
      this.route = parse(next);
    }
  }
}

export const router = new Router();
