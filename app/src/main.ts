import '@kipple/design/styles.css';
import './app.css';
import { mount } from 'svelte';
import App from './App.svelte';
import { native } from './lib/client.ts';

document.documentElement.classList.toggle('native', native);

const target = document.getElementById('app');
if (!target) throw new Error('missing #app');

export default mount(App, { target });
