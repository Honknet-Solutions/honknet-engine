import { defineGameModule } from '@honknet/server';
import { ExampleEvents } from '@example-game/shared';

export default defineGameModule({
  id: 'example-station',

  initialize(commands): void {
    commands.log('info', 'Example game module initialized');
  },

  tick(context): void {
    for (const event of context.events) {
      if (event.name === ExampleEvents.interacted) {
        context.commands.log('debug', `Interaction on tick ${context.tick}`);
      }
    }
  },
});
