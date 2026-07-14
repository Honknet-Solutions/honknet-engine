import './style.css';

import { HonknetStudioApp } from './app';

const root = document.querySelector<HTMLElement>('#app');
if (!root) throw new Error('Missing #app root element.');

new HonknetStudioApp(root);
