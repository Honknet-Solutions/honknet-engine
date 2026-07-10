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

export type EntitySnapshot = {
  net_id: EntityNetId;
  prototype: string;
  position: NetPosition;
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
        movement: Vec2;
      };
    }
  | {
      type: 'Chat';
      data: {
        text: string;
      };
    }
  | {
      type: 'Interact';
      data: {
        target: EntityNetId;
      };
    };

export type ServerMessage =
  | {
      type: 'Welcome';
      data: {
        client_id: string;
        entity_net_id: EntityNetId;
      };
    }
  | {
      type: 'Snapshot';
      data: {
        tick: number;
        last_processed_input_seq: number | null;
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
      type: 'Error';
      data: {
        message: string;
      };
    };