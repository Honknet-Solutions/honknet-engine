export type EntityNetId = number;

export type Vec2 = {
  x: number;
  y: number;
};

export type NetPosition = {
  x: number;
  y: number;
  z: number;
};

export type MapSnapshot = {
  id: string;
  width: number;
  height: number;
  tiles: number[];
};

export type ComponentSnapshot =
  | {
      component: 'Player';
      data: {
        display_name: string;
        online: boolean;
      };
    }
  | {
      component: 'Door';
      data: {
        open: boolean;
      };
    }
  | {
      component: 'Item';
      data: {
        name: string;
      };
    }
  | {
      component: 'Inventory';
      data: {
        items: string[];
      };
    };

export type EntitySnapshot = {
  net_id: EntityNetId;
  prototype: string;
  position: NetPosition;
  components: ComponentSnapshot[];
};

export type ClientMessage =
  | {
      type: 'Hello';
      data: {
        client_version: string;
        identity_id: string;
      };
    }
  | {
      type: 'Input';
      data: {
        seq: number;
        client_tick: number;
        movement: Vec2;
      };
    }
  | {
      type: 'Interact';
      data: {
        target: EntityNetId;
      };
    }
  | {
      type: 'Chat';
      data: {
        text: string;
      };
    };

export type ServerMessage =
  | {
      type: 'Welcome';
      data: {
        client_id: string;
        entity_net_id: EntityNetId;
        map: MapSnapshot;
      };
    }
  | {
      type: 'Snapshot';
      data: {
        tick: number;
        last_processed_input_seq: number | null;
        last_processed_client_tick: number | null;
        entities: EntitySnapshot[];
      };
    }
  | {
      type: 'Chat';
      data: {
        from: string;
        text: string;
      };
    }
  | {
      type: 'System';
      data: {
        text: string;
      };
    }
  | {
      type: 'Error';
      data: {
        message: string;
      };
    };
