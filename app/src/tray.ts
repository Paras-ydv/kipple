import '@kipple/design/styles.css';
import './app.css';
import { mount } from 'svelte';
import { native } from './lib/client.ts';
import TrayApp from './tray/TrayApp.svelte';

document.documentElement.classList.toggle('native', native);

const target = document.getElementById('tray');
if (!target) throw new Error('missing #tray');

export default mount(TrayApp, { target });
