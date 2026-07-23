declare module 'yaml' {
    const YAML: {
        parse(text: string): any;
        stringify(value: any): string;
    };
    export default YAML;
}
