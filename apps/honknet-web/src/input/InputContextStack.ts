export class InputContextStack {
    private contexts: string[] = ['game'];

    public push(context: string): void {
        this.contexts.push(context);
    }

    public pop(): string | undefined {
        if (this.contexts.length > 1) {
            return this.contexts.pop();
        }
        return undefined;
    }

    public current(): string {
        return this.contexts[this.contexts.length - 1];
    }
}
