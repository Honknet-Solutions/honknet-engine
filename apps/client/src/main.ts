import './style.css';

const app = document.querySelector<HTMLDivElement>('#app');

if (!app) {
  throw new Error('Missing #app root element');
}

app.innerHTML = `
  <main class="shell">
    <section class="panel">
      <p class="eyebrow">Open Station</p>
      <h1>Space Station 15</h1>
      <p>Browser-first multiplayer 2D immersive simulation framework.</p>
      <button id="connect">Connect to local server</button>
      <pre id="log">Client booted.</pre>
    </section>
  </main>
`;

const log = document.querySelector<HTMLPreElement>('#log');
const button = document.querySelector<HTMLButtonElement>('#connect');

function writeLog(message: string): void {
  if (!log) return;
  log.textContent += `\n${new Date().toLocaleTimeString()} ${message}`;
}

button?.addEventListener('click', () => {
  writeLog('Transport is not implemented yet. Next step: WebSocket client.');
});
