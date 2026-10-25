// @kipple/client: typed access to the kipple core (SPEC §3.4).

export * from './types.ts';
export { KippleError, isUnavailable, type KippleClient, type StreamOptions } from './client.ts';
export { TauriClient, isTauri } from './tauri.ts';
export { MockClient, entryBytes, NATIVE, type MockOptions } from './mock/index.ts';

import type { KippleClient } from './client.ts';
import { MockClient } from './mock/index.ts';
import { TauriClient, isTauri } from './tauri.ts';

/** TauriClient inside the desktop app, MockClient everywhere else. */
export function createClient(): KippleClient {
  return isTauri() ? new TauriClient() : new MockClient();
}
