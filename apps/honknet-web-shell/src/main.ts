const canvas = document.querySelector<HTMLCanvasElement>('#game')!;
const context = canvas.getContext('2d')!;

let position = 0;
let lastFrame = performance.now();

function frame(now: number): void {
    const deltaSeconds = Math.min(0.1, (now - lastFrame) / 1000);
    lastFrame = now;
    position = (position + deltaSeconds * 120) % canvas.width;

    context.fillStyle = '#050812';
    context.fillRect(0, 0, canvas.width, canvas.height);

    context.fillStyle = '#24e7c4';
    context.fillRect(position, 320, 32, 32);

    requestAnimationFrame(frame);
}

requestAnimationFrame(frame);
