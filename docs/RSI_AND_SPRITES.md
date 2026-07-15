# RSI и спрайты

Поддерживаются обычные PNG/WebP и SS14-подобные `.rsi` каталоги.

```text
player.rsi/
├── meta.json
├── idle.png
└── walk.png
```

`meta.json`:

```json
{
  "version": 1,
  "size": { "x": 32, "y": 32 },
  "states": [
    { "name": "idle", "directions": 4 },
    {
      "name": "walk",
      "directions": 4,
      "delays": [[0.1, 0.1], [0.1, 0.1], [0.1, 0.1], [0.1, 0.1]]
    }
  ]
}
```

Sprite layers поддерживают texture/RSI source, state, direction, color, alpha, scale, offset, rotation, visibility и z index.
