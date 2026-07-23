export class InputActionMap {
    private actionBindings: Map<string, string[]> = new Map();

    public bind(action: string, code: string): void {
        let bindings = this.actionBindings.get(action);
        if (!bindings) {
            bindings = [];
            this.actionBindings.set(action, bindings);
        }
        if (!bindings.includes(code)) {
            bindings.push(code);
        }
    }

    public getBindings(action: string): string[] {
        return this.actionBindings.get(action) || [];
    }
}
