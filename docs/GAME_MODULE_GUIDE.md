# Гайд по созданию игрового модуля

## Что должен знать разработчик

Обычному разработчику контента нужен только TypeScript. Дизайнер может работать через Honknet Studio и YAML. Rust нужен только для изменения самого движка или создания высокопроизводительной native-системы.

## Структура

```text
game/my-game/
├── game.toml
├── server/src/index.ts
├── client/src/index.ts
├── shared/src/index.ts
├── content/
│   ├── prototypes/
│   ├── component-schemas/
│   ├── behaviors/
│   └── ui/
├── resources/
├── localization/
└── maps/
```

## Серверная логика

```ts
import { defineGameModule } from '@honknet/server';

export default defineGameModule({
  id: 'my-game',
  tick(context) {
    for (const event of context.events) {
      if (event.name === 'damage') {
        context.commands.setComponent(
          event.entity!,
          'Health',
          { current: 50, maximum: 100 },
        );
      }
    }
  },
});
```

## Клиентский UI

UI можно собрать в Honknet Studio и сохранить как `.hui.yml`. Для нестандартного поведения регистрируется TypeScript controller через `@honknet/client`.

## Прототипы

```yaml
- type: entity
  id: MyChair
  parent: BaseEntity
  components:
    - type: Sprite
      layers:
        - map: base
          sprite: /Resources/Textures/Furniture/chair.rsi
          state: chair
```

## Компоненты без Rust

Простой replicated component описывается component schema. Build pipeline создаёт validation descriptor и TypeScript type. Native Rust component нужен только для transform, physics, networking или другой hot-path системы.

## Builder и ручная правка

Studio всегда сохраняет человекочитаемые YAML/FTL файлы. Любой документ можно редактировать вручную. Builder не генерирует Rust или TypeScript-мусор.
