export type HuiPrimitive = string | number | boolean | null;

export type HuiBindable<T> = T | string;

export type HuiAction =
  | string
  | HuiSendMessageAction
  | HuiCallControllerAction
  | HuiSetStateAction
  | HuiToggleStateAction
  | HuiOpenWindowAction
  | HuiCloseWindowAction
  | HuiPlaySoundAction
  | HuiSequenceAction;

export type HuiSendMessageAction = {
  type: 'SendMessage';
  message: string;
  arguments?: Record<string, unknown>;
};

export type HuiCallControllerAction = {
  type: 'CallController';
  action: string;
  arguments?: Record<string, unknown>;
};

export type HuiSetStateAction = {
  type: 'SetState';
  path: string;
  value: unknown;
};

export type HuiToggleStateAction = {
  type: 'ToggleState';
  path: string;
};

export type HuiOpenWindowAction = {
  type: 'OpenWindow';
  window: string;
};

export type HuiCloseWindowAction = {
  type: 'CloseWindow';
  window?: string;
};

export type HuiPlaySoundAction = {
  type: 'PlaySound';
  source: string;
};

export type HuiSequenceAction = {
  type: 'Sequence';
  actions: HuiAction[];
};

export type HuiNode = {
  type: string;
  id?: string;
  class?: string;
  styleClass?: string;
  text?: HuiBindable<string>;
  title?: HuiBindable<string>;
  tooltip?: HuiBindable<string>;
  source?: HuiBindable<string>;
  state?: HuiBindable<string>;
  rsiDirection?: number;
  frame?: number;
  animate?: boolean;
  alt?: HuiBindable<string>;
  value?: HuiBindable<unknown>;
  items?: HuiBindable<unknown[]>;
  selected?: HuiBindable<string | number | null>;
  checked?: HuiBindable<boolean>;
  pressed?: HuiBindable<boolean>;
  visible?: HuiBindable<boolean>;
  enabled?: HuiBindable<boolean>;

  width?: number | string;
  height?: number | string;
  size?: [number | string, number | string];
  minWidth?: number;
  minHeight?: number;
  maxWidth?: number;
  maxHeight?: number;
  x?: number;
  y?: number;
  grow?: number;
  shrink?: number;
  margin?: number | string;
  padding?: number | string;
  gap?: number;
  rowGap?: number;
  columnGap?: number;
  columns?: number;
  rows?: number;
  columnSpan?: number;
  rowSpan?: number;
  orientation?: 'horizontal' | 'vertical';
  direction?: 'row' | 'column';
  alignItems?: 'start' | 'center' | 'end' | 'stretch';
  justifyContent?: 'start' | 'center' | 'end' | 'space-between' | 'space-around' | 'space-evenly';
  selfAlign?: 'auto' | 'start' | 'center' | 'end' | 'stretch';
  wrap?: boolean;
  overflow?: 'visible' | 'hidden' | 'auto' | 'scroll';
  clip?: boolean;
  opacity?: number;
  zIndex?: number;
  anchorLeft?: boolean;
  anchorRight?: boolean;
  anchorTop?: boolean;
  anchorBottom?: boolean;

  placeholder?: HuiBindable<string>;
  multiline?: boolean;
  password?: boolean;
  maxLength?: number;
  minimum?: number;
  maximum?: number;
  step?: number;
  toggle?: boolean;
  icon?: HuiBindable<string>;
  fit?: 'contain' | 'cover' | 'fill' | 'none';
  keepAspect?: boolean;
  tabTitle?: HuiBindable<string>;
  activeTab?: HuiBindable<number>;
  split?: number;

  onClick?: HuiAction;
  onDoubleClick?: HuiAction;
  onChange?: HuiAction;
  onSubmit?: HuiAction;
  onSelected?: HuiAction;
  onValueChanged?: HuiAction;
  onFocus?: HuiAction;
  onBlur?: HuiAction;

  children?: HuiNode[];
  [key: string]: unknown;
};

export type HuiContext = {
  state: Record<string, unknown>;
  localize: (key: string) => string;
  action: (name: string, payload?: unknown) => void;
  sendMessage?: (message: string, payload?: Record<string, unknown>) => void;
  setState?: (path: string, value: unknown) => void;
  openWindow?: (window: string) => void;
  closeWindow?: (window?: string) => void;
  playSound?: (source: string) => void;
  resolveResource?: (source: string) => string | Promise<string>;
};

export type HuiRenderOptions = {
  document?: Document;
  designMode?: boolean;
  getNodeKey?: (node: HuiNode) => string | undefined;
  onNodeCreated?: (node: HuiNode, element: HTMLElement) => void;
  onNodePointerDown?: (node: HuiNode, event: PointerEvent, element: HTMLElement) => void;
  onNodeClick?: (node: HuiNode, event: MouseEvent, element: HTMLElement) => void;
  onNodeDoubleClick?: (node: HuiNode, event: MouseEvent, element: HTMLElement) => void;
};

export type HuiPropertyEditor =
  | 'text'
  | 'number'
  | 'boolean'
  | 'select'
  | 'dimension'
  | 'resource'
  | 'binding'
  | 'action'
  | 'items';

export type HuiPropertySchema = {
  name: string;
  label: string;
  category: 'Content' | 'Layout' | 'Appearance' | 'Behavior' | 'Events';
  editor: HuiPropertyEditor;
  bindable?: boolean;
  defaultValue?: unknown;
  minimum?: number;
  maximum?: number;
  step?: number;
  options?: readonly string[];
  hint?: string;
};

export type HuiControlSchema = {
  type: string;
  label: string;
  group: 'Layout' | 'Controls' | 'Game';
  container: boolean;
  freeformContainer?: boolean;
  maximumChildren?: number;
  properties: readonly HuiPropertySchema[];
  defaults: Readonly<Record<string, unknown>>;
};

export type HuiValidationIssue = {
  severity: 'error' | 'warning' | 'info';
  path: string;
  message: string;
};
