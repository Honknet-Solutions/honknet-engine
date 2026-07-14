export const PROTOCOL_VERSION = 1;

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

export type SpriteLayerSnapshot = {
  key: string;
  source:
    | { kind: 'texture'; path: string }
    | { kind: 'rsi'; path: string; state: string };
  visible: boolean;
  color: [number, number, number, number];
  scale: [number, number];
  offset: [number, number];
  rotation: number;
  z_index: number;
  direction: number;
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
    }
  | {
      component: 'Sprite';
      data: {
        layers: SpriteLayerSnapshot[];
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
        protocol_version: number;
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
    }
  | {
      type: 'SnapshotAck';
      data: {
        tick: number;
      };
    }
  | {
      type: 'UiAction';
      data: {
        session_id: string;
        action: string;
        payload: unknown;
      };
    };

export type ServerMessage =
  | {
      type: 'Welcome';
      data: {
        protocol_version: number;
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
      type: 'StateDelta';
      data: {
        tick: number;
        last_processed_input_seq: number | null;
        last_processed_client_tick: number | null;
        spawns: EntitySnapshot[];
        updates: EntitySnapshot[];
        despawns: EntityNetId[];
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
      type: 'UiOpen';
      data: {
        session_id: string;
        key: string;
        target: EntityNetId;
        state: unknown;
      };
    }
  | {
      type: 'UiState';
      data: {
        session_id: string;
        state: unknown;
      };
    }
  | {
      type: 'UiClose';
      data: {
        session_id: string;
      };
    }
  | {
      type: 'PlaySound';
      data: {
        path: string;
        position: NetPosition | null;
      };
    }
  | {
      type: 'Error';
      data: {
        message: string;
      };
    };
