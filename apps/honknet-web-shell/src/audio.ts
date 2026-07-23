export type WebSound = {
    url: string;
    position?: [number, number];
    volume: number;
    loop: boolean;
    bus: string;
    maxDistance: number;
};

export class WebAudioBackend {
    private readonly context = new AudioContext();
    private listener: [number, number] = [0, 0];
    private readonly buses = new Map<string, GainNode>();

    setListener(position: [number, number]): void {
        this.listener = position;
    }

    setBus(name: string, volume: number): void {
        let gain = this.buses.get(name);

        if (!gain) {
            gain = this.context.createGain();
            gain.connect(this.context.destination);
            this.buses.set(name, gain);
        }

        gain.gain.value = Math.max(0, volume);
    }

    async play(sound: WebSound): Promise<AudioBufferSourceNode> {
        const response = await fetch(sound.url);
        const arrayBuffer = await response.arrayBuffer();
        const buffer = await this.context.decodeAudioData(arrayBuffer);

        const source = this.context.createBufferSource();
        source.buffer = buffer;
        source.loop = sound.loop;

        const gain = this.context.createGain();
        const distance = sound.position
            ? Math.hypot(
                sound.position[0] - this.listener[0],
                sound.position[1] - this.listener[1],
            )
            : 0;

        gain.gain.value = sound.volume
            * Math.max(0, 1 - distance / Math.max(0.1, sound.maxDistance));

        source.connect(gain);

        let bus = this.buses.get(sound.bus);
        if (!bus) {
            this.setBus(sound.bus, 1);
            bus = this.buses.get(sound.bus)!;
        }

        gain.connect(bus);
        source.start();
        return source;
    }
}
