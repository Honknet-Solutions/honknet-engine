export type ToolMode = 
  | 'project' 
  | 'map' 
  | 'ui' 
  | 'prototype' 
  | 'profiler' 
  | 'network' 
  | 'migration';

export type Kind = 'map' | 'prototype' | 'ui' | 'animation' | 'localization' | 'resource' | 'replay' | 'network' | 'manifest';

export interface Doc {
    id: string;
    name: string;
    kind: Kind;
    text: string;
    dirty: boolean;
}

export interface ProjectManifest {
    id: string;
    name: string;
    version: string;
    engineVersion: string;
    protocolVersion: number;
    contentSchemaVersion: number;
    targets: {
        server: boolean;
        desktop: boolean;
        web: boolean;
        docker: boolean;
    };
    packages: Array<{ name: string; version: string; status: 'ok' | 'outdated' | 'missing' }>;
}

export interface TileDefinition {
    id: string;
    name: string;
    solid: boolean;
    friction: number;
    color: string;
    texture?: string;
}

export interface MapGridChunk {
    x: number;
    y: number;
    revision: number;
    tiles: number[];
}

export interface MapEntityPlacement {
    id: string;
    prototypeId: string;
    x: number;
    y: number;
    layer: string;
}

export interface MapData {
    id: string;
    tileSize: number;
    tiles: TileDefinition[];
    chunks: MapGridChunk[];
    entities: MapEntityPlacement[];
}

export interface HuiNode {
    id: string;
    type: 'Window' | 'Panel' | 'Label' | 'Button' | 'TextInput' | 'VirtualList' | 'ProgressBar' | 'Viewport';
    title?: string;
    text?: string;
    binding?: string;
    flexDirection?: 'row' | 'column';
    width?: string;
    height?: string;
    padding?: string;
    backgroundColor?: string;
    color?: string;
    children?: HuiNode[];
}

export interface ComponentSchemaField {
    name: string;
    type: 'string' | 'number' | 'boolean' | 'vec2' | 'enum';
    defaultValue: any;
    description: string;
}

export interface ComponentSchema {
    id: string;
    rustType: string;
    storage: 'packed' | 'sparse';
    fields: ComponentSchemaField[];
}

export interface PrototypeDefinition {
    id: string;
    name: string;
    parents: string[];
    isAbstract: boolean;
    components: Record<string, Record<string, any>>;
}

export interface SystemTiming {
    name: string;
    phase: string;
    durationMs: number;
    cpuPercentage: number;
}

export interface ProfilerFrame {
    tps: number;
    frameTimeMs: number;
    entitiesCount: number;
    physicsContacts: number;
    memoryMb: number;
    timings: SystemTiming[];
}

export interface PacketInspect {
    id: number;
    kind: string;
    channel: 'ReliableOrdered' | 'ReliableUnordered' | 'UnreliableSequenced' | 'Control';
    bytes: number;
    seq: number;
    ack: number;
    timestamp: string;
}

export interface PvsRegion {
    id: string;
    entitiesCount: number;
    byteBudgetKbps: number;
    active: boolean;
}
