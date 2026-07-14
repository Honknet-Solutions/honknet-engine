import { defineClientModule } from '@honknet/client';

export default defineClientModule({
  id: 'example-station',
  register(context): void {
    context.ui.register('example.status', (session, send) => {
      const root = document.createElement('section');
      root.className = 'hui-window';
      const title = document.createElement('h2');
      title.textContent = 'Example Status';
      const button = document.createElement('button');
      button.textContent = 'Ping server';
      button.addEventListener('click', () => send(session.sessionId, 'ping', null));
      root.append(title, button);
      return root;
    });
  },
});
