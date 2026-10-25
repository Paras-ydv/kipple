// One client for the whole app: TauriClient inside Tauri, MockClient in a browser.
import { createClient, isTauri, type KippleClient } from '@kipple/client';

export const client: KippleClient = createClient();
export const native = isTauri();
