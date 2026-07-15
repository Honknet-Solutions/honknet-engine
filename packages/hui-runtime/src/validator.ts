import { isHuiBinding } from './bindings';
import { getHuiControlSchema, isHuiContainer } from './schema';
import type { HuiNode, HuiValidationIssue } from './types';

const EVENT_PROPERTIES = new Set([
  'onClick',
  'onDoubleClick',
  'onChange',
  'onSubmit',
  'onSelected',
  'onValueChanged',
  'onFocus',
  'onBlur',
]);

export function validateHuiDocument(root: HuiNode): HuiValidationIssue[] {
  const issues: HuiValidationIssue[] = [];
  const ids = new Map<string, string>();

  const visit = (node: HuiNode, path: string, parent: HuiNode | undefined): void => {
    const control = getHuiControlSchema(node.type);
    if (!control) {
      issues.push({ severity: 'error', path, message: `Unsupported HUI control type: ${node.type}` });
    }

    if (node.id) {
      const previousPath = ids.get(node.id);
      if (previousPath) issues.push({ severity: 'error', path, message: `Duplicate ID '${node.id}'. First used at ${previousPath}.` });
      else ids.set(node.id, path);
    }

    const children = node.children ?? [];
    if (children.length > 0 && !isHuiContainer(node.type)) {
      issues.push({ severity: 'error', path, message: `${node.type} cannot contain child controls.` });
    }
    if (control?.maximumChildren !== undefined && children.length > control.maximumChildren) {
      issues.push({ severity: 'error', path, message: `${node.type} supports at most ${control.maximumChildren} child controls.` });
    }

    if (parent && (parent.type === 'Canvas' || parent.type === 'Overlay')) {
      if (typeof node.x !== 'number' || typeof node.y !== 'number') {
        issues.push({ severity: 'warning', path, message: `Freeform child should define numeric x and y positions.` });
      }
    }

    const knownProperties = new Set(control?.properties.map((property) => property.name) ?? []);
    knownProperties.add('type');
    knownProperties.add('children');
    knownProperties.add('tabTitle');
    knownProperties.add('size');
    for (const [property, value] of Object.entries(node)) {
      if (property.startsWith('_')) continue;
      if (!knownProperties.has(property) && !EVENT_PROPERTIES.has(property)) {
        issues.push({ severity: 'warning', path: `${path}.${property}`, message: `Property '${property}' is not declared by the ${node.type} schema.` });
      }
      if (typeof value === 'string' && value.startsWith('$') && !isHuiBinding(value)) {
        issues.push({ severity: 'error', path: `${path}.${property}`, message: `Invalid binding '${value}'. Expected $state.path, $selected.path or $event.path.` });
      }
    }

    for (const property of control?.properties ?? []) {
      const value = node[property.name];
      if (value === undefined) continue;
      if (property.editor === 'number' && typeof value !== 'number' && !isHuiBinding(value)) {
        issues.push({ severity: 'error', path: `${path}.${property.name}`, message: `Expected a number or binding.` });
      }
      if (property.editor === 'boolean' && typeof value !== 'boolean' && !isHuiBinding(value)) {
        issues.push({ severity: 'error', path: `${path}.${property.name}`, message: `Expected a boolean or binding.` });
      }
      if (property.options && typeof value === 'string' && !isHuiBinding(value) && !property.options.includes(value)) {
        issues.push({ severity: 'error', path: `${path}.${property.name}`, message: `Unsupported value '${value}'.` });
      }
    }

    children.forEach((child, index) => visit(child, `${path}.children[${index}]`, node));
  };

  visit(root, 'root', undefined);
  if (issues.length === 0) issues.push({ severity: 'info', path: 'root', message: 'HUI document is valid and supported by the shared runtime.' });
  return issues;
}
