import type { HuiControlSchema, HuiPropertySchema } from './types';

const content = (name: string, label: string, editor: HuiPropertySchema['editor'], extra: Partial<HuiPropertySchema> = {}): HuiPropertySchema => ({
  name,
  label,
  category: 'Content',
  editor,
  ...extra,
});

const layout = (name: string, label: string, editor: HuiPropertySchema['editor'], extra: Partial<HuiPropertySchema> = {}): HuiPropertySchema => ({
  name,
  label,
  category: 'Layout',
  editor,
  ...extra,
});

const appearance = (name: string, label: string, editor: HuiPropertySchema['editor'], extra: Partial<HuiPropertySchema> = {}): HuiPropertySchema => ({
  name,
  label,
  category: 'Appearance',
  editor,
  ...extra,
});

const behavior = (name: string, label: string, editor: HuiPropertySchema['editor'], extra: Partial<HuiPropertySchema> = {}): HuiPropertySchema => ({
  name,
  label,
  category: 'Behavior',
  editor,
  ...extra,
});

const event = (name: string, label: string): HuiPropertySchema => ({
  name,
  label,
  category: 'Events',
  editor: 'action',
});

const COMMON: readonly HuiPropertySchema[] = [
  content('id', 'ID', 'text', { hint: 'Stable identifier used by controllers and bindings.' }),
  behavior('visible', 'Visible', 'binding', { bindable: true, defaultValue: true }),
  behavior('enabled', 'Enabled', 'binding', { bindable: true, defaultValue: true }),
  content('tooltip', 'Tooltip', 'binding', { bindable: true }),
  layout('width', 'Width', 'dimension'),
  layout('height', 'Height', 'dimension'),
  layout('minWidth', 'Minimum width', 'number', { minimum: 0 }),
  layout('minHeight', 'Minimum height', 'number', { minimum: 0 }),
  layout('maxWidth', 'Maximum width', 'number', { minimum: 0 }),
  layout('maxHeight', 'Maximum height', 'number', { minimum: 0 }),
  layout('grow', 'Grow', 'number', { minimum: 0, maximum: 100, step: 1, defaultValue: 0 }),
  layout('shrink', 'Shrink', 'number', { minimum: 0, maximum: 100, step: 1, defaultValue: 1 }),
  layout('selfAlign', 'Self alignment', 'select', { options: ['auto', 'start', 'center', 'end', 'stretch'] }),
  layout('margin', 'Margin', 'text', { hint: 'CSS-like: 8 or 8 12 or 8 12 16 12.' }),
  appearance('styleClass', 'Style class', 'text'),
  appearance('opacity', 'Opacity', 'number', { minimum: 0, maximum: 1, step: 0.05, defaultValue: 1 }),
  appearance('zIndex', 'Z index', 'number', { minimum: -1000, maximum: 1000, step: 1, defaultValue: 0 }),
];

const FREEFORM: readonly HuiPropertySchema[] = [
  layout('x', 'Position X', 'number', { step: 1, defaultValue: 0 }),
  layout('y', 'Position Y', 'number', { step: 1, defaultValue: 0 }),
  layout('anchorLeft', 'Anchor left', 'boolean', { defaultValue: false }),
  layout('anchorRight', 'Anchor right', 'boolean', { defaultValue: false }),
  layout('anchorTop', 'Anchor top', 'boolean', { defaultValue: false }),
  layout('anchorBottom', 'Anchor bottom', 'boolean', { defaultValue: false }),
];

const CONTAINER: readonly HuiPropertySchema[] = [
  layout('padding', 'Padding', 'text', { hint: 'CSS-like: 8 or 8 12 or 8 12 16 12.' }),
  layout('gap', 'Gap', 'number', { minimum: 0, maximum: 256, defaultValue: 8 }),
  layout('alignItems', 'Align children', 'select', { options: ['start', 'center', 'end', 'stretch'], defaultValue: 'stretch' }),
  layout('justifyContent', 'Justify children', 'select', { options: ['start', 'center', 'end', 'space-between', 'space-around', 'space-evenly'], defaultValue: 'start' }),
  layout('wrap', 'Wrap children', 'boolean', { defaultValue: false }),
  layout('overflow', 'Overflow', 'select', { options: ['visible', 'hidden', 'auto', 'scroll'], defaultValue: 'visible' }),
  behavior('clip', 'Clip content', 'boolean', { defaultValue: false }),
];

function schema(
  type: string,
  label: string,
  group: HuiControlSchema['group'],
  container: boolean,
  properties: readonly HuiPropertySchema[],
  defaults: Readonly<Record<string, unknown>> = {},
  options: Pick<HuiControlSchema, 'freeformContainer' | 'maximumChildren'> = {},
): HuiControlSchema {
  return {
    type,
    label,
    group,
    container,
    properties: [...COMMON, ...properties],
    defaults,
    ...options,
  };
}

const schemas: HuiControlSchema[] = [
  schema('Window', 'Window', 'Layout', true, [
    content('title', 'Window title', 'binding', { bindable: true }),
    ...CONTAINER,
  ], { width: 640, height: 420, padding: 16, gap: 12, children: [] }),
  schema('Row', 'Row', 'Layout', true, CONTAINER, { gap: 8, alignItems: 'center', children: [] }),
  schema('Column', 'Column', 'Layout', true, CONTAINER, { gap: 8, alignItems: 'stretch', children: [] }),
  schema('Grid', 'Grid', 'Layout', true, [
    ...CONTAINER,
    layout('columns', 'Columns', 'number', { minimum: 1, maximum: 64, step: 1, defaultValue: 2 }),
    layout('rows', 'Rows', 'number', { minimum: 0, maximum: 64, step: 1 }),
    layout('columnGap', 'Column gap', 'number', { minimum: 0, maximum: 256 }),
    layout('rowGap', 'Row gap', 'number', { minimum: 0, maximum: 256 }),
  ], { columns: 2, gap: 8, children: [] }),
  schema('Panel', 'Panel', 'Layout', true, CONTAINER, { padding: 12, gap: 8, children: [] }),
  schema('Canvas', 'Freeform Canvas', 'Layout', true, [
    ...CONTAINER,
    ...FREEFORM,
  ], { width: 'fill', height: 360, padding: 0, children: [] }, { freeformContainer: true }),
  schema('Overlay', 'Overlay', 'Layout', true, [
    ...CONTAINER,
    ...FREEFORM,
  ], { width: 'fill', height: 'fill', padding: 0, children: [] }, { freeformContainer: true }),
  schema('ScrollContainer', 'Scroll Container', 'Layout', true, [
    ...CONTAINER,
    layout('orientation', 'Scroll direction', 'select', { options: ['vertical', 'horizontal'], defaultValue: 'vertical' }),
  ], { overflow: 'auto', children: [] }, { maximumChildren: 1 }),
  schema('SplitContainer', 'Split Container', 'Layout', true, [
    ...CONTAINER,
    layout('orientation', 'Orientation', 'select', { options: ['horizontal', 'vertical'], defaultValue: 'horizontal' }),
    layout('split', 'Divider position (%)', 'number', { minimum: 5, maximum: 95, step: 1, defaultValue: 50 }),
  ], { orientation: 'horizontal', split: 50, children: [] }, { maximumChildren: 2 }),
  schema('TabContainer', 'Tab Container', 'Layout', true, [
    ...CONTAINER,
    content('activeTab', 'Active tab', 'binding', { bindable: true, defaultValue: 0 }),
  ], { activeTab: 0, children: [] }),
  schema('Spacer', 'Spacer', 'Layout', false, [], { grow: 1, minWidth: 8, minHeight: 8 }),

  schema('Label', 'Label', 'Controls', false, [
    content('text', 'Text / FTL key', 'binding', { bindable: true, defaultValue: 'Label' }),
    appearance('textAlign', 'Text alignment', 'select', { options: ['left', 'center', 'right'] }),
    appearance('fontSize', 'Font size', 'number', { minimum: 8, maximum: 128, step: 1 }),
    behavior('wrapText', 'Wrap text', 'boolean', { defaultValue: true }),
  ], { text: 'Label' }),
  schema('Button', 'Button', 'Controls', false, [
    content('text', 'Text / FTL key', 'binding', { bindable: true, defaultValue: 'Button' }),
    content('icon', 'Icon', 'resource', { bindable: true }),
    behavior('toggle', 'Toggle mode', 'boolean', { defaultValue: false }),
    behavior('pressed', 'Pressed', 'binding', { bindable: true, defaultValue: false }),
    event('onClick', 'On click'),
    event('onDoubleClick', 'On double click'),
  ], { text: 'Button', onClick: 'action' }),
  schema('Image', 'Image', 'Controls', false, [
    content('source', 'Image / RSI resource', 'resource', { bindable: true }),
    content('state', 'RSI state', 'binding', { bindable: true }),
    content('alt', 'Alternative text', 'binding', { bindable: true }),
    behavior('rsiDirection', 'RSI direction', 'number', { minimum: 0, maximum: 7, step: 1, defaultValue: 0 }),
    behavior('frame', 'RSI frame', 'number', { minimum: 0, step: 1, defaultValue: 0 }),
    behavior('animate', 'Animate RSI', 'boolean', { defaultValue: true }),
    appearance('fit', 'Fit', 'select', { options: ['contain', 'cover', 'fill', 'none'], defaultValue: 'contain' }),
    appearance('keepAspect', 'Keep aspect', 'boolean', { defaultValue: true }),
  ], { source: '/Resources/Textures/error.rsi', state: 'error', animate: true, fit: 'contain', keepAspect: true }),
  schema('TextInput', 'Text Input', 'Controls', false, [
    content('value', 'Value', 'binding', { bindable: true, defaultValue: '' }),
    content('placeholder', 'Placeholder / FTL key', 'binding', { bindable: true }),
    behavior('multiline', 'Multiline', 'boolean', { defaultValue: false }),
    behavior('password', 'Password', 'boolean', { defaultValue: false }),
    behavior('maxLength', 'Maximum length', 'number', { minimum: 0, maximum: 100000, step: 1 }),
    event('onChange', 'On change'),
    event('onSubmit', 'On submit'),
    event('onFocus', 'On focus'),
    event('onBlur', 'On blur'),
  ], { value: '' }),
  schema('Checkbox', 'Checkbox', 'Controls', false, [
    content('text', 'Text / FTL key', 'binding', { bindable: true, defaultValue: 'Checkbox' }),
    content('checked', 'Checked', 'binding', { bindable: true, defaultValue: false }),
    event('onChange', 'On change'),
  ], { text: 'Checkbox', checked: false }),
  schema('Slider', 'Slider', 'Controls', false, [
    content('value', 'Value', 'binding', { bindable: true, defaultValue: 50 }),
    behavior('minimum', 'Minimum', 'number', { defaultValue: 0 }),
    behavior('maximum', 'Maximum', 'number', { defaultValue: 100 }),
    behavior('step', 'Step', 'number', { minimum: 0.0001, defaultValue: 1 }),
    layout('orientation', 'Orientation', 'select', { options: ['horizontal', 'vertical'], defaultValue: 'horizontal' }),
    event('onValueChanged', 'On value changed'),
  ], { value: 50, minimum: 0, maximum: 100, step: 1 }),
  schema('ProgressBar', 'Progress Bar', 'Controls', false, [
    content('value', 'Value', 'binding', { bindable: true, defaultValue: 50 }),
    behavior('minimum', 'Minimum', 'number', { defaultValue: 0 }),
    behavior('maximum', 'Maximum', 'number', { defaultValue: 100 }),
    behavior('showValue', 'Show value', 'boolean', { defaultValue: true }),
  ], { value: 50, minimum: 0, maximum: 100 }),
  schema('List', 'List', 'Controls', false, [
    content('items', 'Items', 'items', { bindable: true, defaultValue: '$state.items' }),
    content('selected', 'Selected', 'binding', { bindable: true }),
    event('onSelected', 'On selection changed'),
  ], { items: '$state.items', height: 180 }),
  schema('Dropdown', 'Dropdown', 'Controls', false, [
    content('items', 'Options', 'items', { bindable: true, defaultValue: '$state.options' }),
    content('selected', 'Selected', 'binding', { bindable: true }),
    event('onSelected', 'On selected'),
  ], { items: '$state.options' }),

  schema('EntityView', 'Entity View', 'Game', false, [
    content('value', 'Entity', 'binding', { bindable: true, defaultValue: '$state.entity' }),
  ], { value: '$state.entity', width: 180, height: 180 }),
  schema('InventoryGrid', 'Inventory Grid', 'Game', false, [
    content('items', 'Inventory items', 'items', { bindable: true, defaultValue: '$state.inventory' }),
    layout('columns', 'Columns', 'number', { minimum: 1, maximum: 20, step: 1, defaultValue: 6 }),
    event('onSelected', 'On slot selected'),
  ], { items: '$state.inventory', columns: 6, height: 240 }),
  schema('PaperDoll', 'Paper Doll', 'Game', false, [
    content('value', 'Character', 'binding', { bindable: true, defaultValue: '$state.character' }),
  ], { value: '$state.character', width: 220, height: 320 }),
  schema('MapView', 'Map View', 'Game', false, [
    content('value', 'Map state', 'binding', { bindable: true, defaultValue: '$state.map' }),
  ], { value: '$state.map', width: 320, height: 240 }),
  schema('ChatBox', 'Chat Box', 'Game', false, [
    content('items', 'Messages', 'items', { bindable: true, defaultValue: '$state.messages' }),
    event('onSubmit', 'On message submitted'),
  ], { items: '$state.messages', height: 220 }),
];

export const HUI_CONTROL_SCHEMAS: ReadonlyMap<string, HuiControlSchema> = new Map(schemas.map((entry) => [entry.type, entry]));

export function getHuiControlSchema(type: string): HuiControlSchema | undefined {
  return HUI_CONTROL_SCHEMAS.get(type);
}

export function listHuiControlSchemas(): readonly HuiControlSchema[] {
  return schemas;
}

export function createHuiNode(type: string): Record<string, unknown> {
  const control = getHuiControlSchema(type);
  return { type, ...(control ? structuredClone(control.defaults) : {}) };
}

export function isHuiContainer(type: string): boolean {
  return getHuiControlSchema(type)?.container ?? false;
}

export function isHuiFreeformContainer(type: string): boolean {
  return getHuiControlSchema(type)?.freeformContainer ?? false;
}
