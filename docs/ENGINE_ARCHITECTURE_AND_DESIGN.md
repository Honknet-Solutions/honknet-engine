# Honknet Runtime — полный дизайн-документ и архитектура

**Проект:** `honknet-engine`  
**Организация:** `Honknet Solutions`  
**Документ:** Full Product Design Document / Architecture Design Document  
**Версия:** 1.0  
**Статус:** проектная спецификация конечного продукта  
**Язык документа:** RU  

---

## 0. Краткое определение проекта

**Honknet Runtime** — это browser-first open-source framework/engine для многопользовательских 2D immersive simulation sandbox/RP игр.

Проект не является форком Space Station 13, Space Station 14, RobustToolbox или любой другой существующей кодовой базы. Он не завязан на космические станции как жанр, на атмосферу, шаттлы, отделы станции, разгерметизацию или любые другие SS13/SS14-специфичные механики.

Название **Honknet Runtime** сохраняется как бренд и историческое имя проекта, но технически `honknet-engine` должен быть универсальной платформой для создания разных 2D сетевых симуляционных игр:

- киберпанковские города;
- космические станции;
- подземные бункеры;
- фэнтези-поселения;
- колонии;
- SCP-комплексы;
- метро/убежища;
- постапокалиптические города;
- любые другие persistent/sandbox/RP миры.

Главный принцип: **Core не знает про сеттинг**. Сеттинг появляется только в модулях и конкретных игровых сборках.

---

## 1. Главная цель конечного продукта

Цель проекта — создать полноценную web-first платформу для игр уровня SS13/SS14 по глубине взаимодействий, но без наследия и жёстких архитектурных привязок к космической станции.

Конечный продукт должен позволять разработчику или команде создать собственную 2D multiplayer simulation игру, подключив нужные модули и написав собственный контент.

Итоговая платформа должна включать:

1. **Авторитетный сервер симуляции** на Rust.
2. **Браузерный клиент** на TypeScript.
3. **2D renderer** для тайлов, сущностей, света, эффектов и UI-overlays.
4. **Сетевой слой** для real-time multiplayer.
5. **ECS/world simulation core**.
6. **Систему карт, чанков, зон и уровней**.
7. **Систему прототипов/контента**.
8. **Модульную архитектуру игровых систем**.
9. **UI framework** для сложных игровых интерфейсов.
10. **Инструменты разработчика**.
11. **Редактор карт и контента**.
12. **Админские инструменты**.
13. **Систему сохранения мира**.
14. **Систему прав, ролей, серверной модерации**.
15. **Документацию и open-source governance**.
16. **Флагманскую showcase-сборку**, доказывающую, что движок работает в реальной игре.

---

## 2. Ключевая философия

### 2.1. Browser-first

Клиент должен запускаться в браузере без установки лаунчера и отдельного клиента.

Игрок должен иметь возможность открыть ссылку, авторизоваться и подключиться к серверу.

Целевой пользовательский путь:

```text
Игрок получает ссылку
↓
Открывает сайт
↓
Проходит авторизацию / выбирает сервер
↓
Создаёт или выбирает персонажа
↓
Нажимает Play
↓
Подключается к игровому миру
```

Браузерный клиент должен быть не временной демкой, а полноценным основным клиентом проекта.

### 2.2. Server-authoritative

Сервер всегда является источником правды.

Клиент не решает:

- попадание урона;
- результат атаки;
- состояние здоровья;
- инвентарь;
- владение предметами;
- открытие дверей;
- прохождение коллизий;
- экономику;
- права доступа;
- спавн сущностей;
- смерть;
- сохранение мира;
- видимость критически важных объектов.

Клиент может делать:

- локальное предсказание движения;
- интерполяцию;
- визуальные эффекты;
- UI-анимации;
- предзагрузку ассетов;
- локальные overlay-подсказки;
- временное отображение ожидающих действий до подтверждения сервера.

### 2.3. Core без сеттинга

Core не должен содержать конкретные механики космоса, киберпанка, фэнтези или любого другого сеттинга.

В Core допустимы только универсальные понятия:

- Entity;
- Component;
- System;
- Transform;
- Map;
- Tile;
- Chunk;
- Zone;
- Physics body;
- Collider;
- Network identity;
- Asset reference;
- Input action;
- Event;
- Resource;
- Permission;
- Serialization;
- Save/load.

В Core запрещено помещать:

- атмосферу как обязательную механику;
- космос;
- шаттлы;
- отделы станции;
- конкретные роли;
- конкретную медицину;
- конкретные органы;
- конкретные виды оружия;
- лорные фракции;
- экономику конкретного сеттинга;
- импланты конкретного сеттинга;
- Netrunning как базовый engine feature;
- любые механики, которые нужны только одной игре.

### 2.4. Модульность

Любая крупная игровая система должна быть модулем.

Примеры модулей:

```text
@open-station/interaction
@open-station/inventory
@open-station/chat
@open-station/combat
@open-station/medical
@open-station/atmosphere
@open-station/power
@open-station/vehicles
@open-station/factions
@open-station/economy
@open-station/dialogue
@open-station/ai
@open-station/cyberpunk
@open-station/space-station
```

Модуль может добавлять:

- компоненты;
- системы;
- прототипы;
- события;
- UI;
- команды;
- permissions;
- контент;
- редакторские панели;
- debug-инструменты;
- серверные конфиги.

### 2.5. Showcase ведёт Core

Core не должен разрабатываться в вакууме.

Флагманская сборка нужна не как побочный проект, а как главный тест архитектуры.

Рекомендуемый первый showcase:

```text
Night City 2045 Web
```

Причина: киберпанковский город заставляет движок быть универсальным, а не повторять структуру космической станции.

Showcase должен проверять:

- городские карты;
- многоэтажные здания;
- зоны города;
- фракции;
- NPC;
- предметы;
- оружие;
- торговлю;
- импланты;
- транспорт;
- приватные интерьеры;
- публичные пространства;
- roleplay-чаты;
- админку;
- persistent world;
- сложный UI.

---

## 3. Высокоуровневая архитектура

```text
+------------------------------------------------------------+
|                     Browser Client                         |
|  TypeScript / WebGL-WebGPU Renderer / HTML-CSS UI          |
|                                                            |
|  - Rendering                                                |
|  - UI                                                       |
|  - Input                                                    |
|  - Prediction                                               |
|  - Interpolation                                            |
|  - Audio                                                    |
|  - Asset cache                                              |
+-------------------------+----------------------------------+
                          |
                          | WebSocket / WebTransport
                          |
+-------------------------v----------------------------------+
|                    Rust Game Server                         |
|                                                            |
|  - ECS World                                                |
|  - Simulation Scheduler                                     |
|  - Networking                                               |
|  - Replication                                              |
|  - Physics / Collision                                      |
|  - Map / Chunk / Zone System                                |
|  - Permissions                                              |
|  - Persistence                                              |
|  - Module Runtime                                           |
+-------------------------+----------------------------------+
                          |
                          | Data / Assets / Config / DB
                          |
+-------------------------v----------------------------------+
|                    Content & Tools                          |
|                                                            |
|  - Prototype database                                       |
|  - Maps                                                     |
|  - Asset packs                                              |
|  - Modules                                                  |
|  - Editor                                                   |
|  - Admin tools                                              |
|  - Documentation                                            |
+------------------------------------------------------------+
```

---

## 4. Технологическая модель

### 4.1. Сервер

Основной сервер пишется на **Rust**.

Причины выбора:

- высокая производительность;
- контроль памяти;
- отсутствие garbage collector pauses;
- безопасная многопоточность;
- хорошая база для долгоживущего сервера;
- удобная экосистема для ECS, async networking, serialization, testing;
- возможность вынести тяжёлые системы в отдельные worker-пулы.

Сервер должен быть авторитетным и не зависеть от браузерного клиента для критической логики.

### 4.2. Клиент

Основной клиент пишется на **TypeScript**.

Клиент запускается в браузере и отвечает за:

- подключение к серверу;
- рендер мира;
- UI;
- ввод;
- звук;
- локальный asset cache;
- client prediction;
- interpolation;
- rendering debug overlays;
- отображение сетевых состояний.

### 4.3. Сетевой транспорт

Итоговая архитектура должна поддерживать два транспорта:

1. **WebSocket** — базовый, простой, совместимый fallback.
2. **WebTransport** — основной high-performance транспорт для будущего real-time режима.

WebSocket нужен для:

- совместимости;
- простого старта;
- fallback-режима;
- админских панелей;
- dev tools;
- серверов, где WebTransport сложно поднять.

WebTransport нужен для:

- низкой задержки;
- разделения надёжных и ненадёжных каналов;
- state replication;
- input stream;
- datagram-like обновлений;
- уменьшения head-of-line blocking проблем, характерных для TCP-подобного потока.

### 4.4. Renderer

Клиентский renderer должен быть самостоятельным слоем.

Варианты реализации:

- на старте: PixiJS/WebGL или собственная WebGL-обёртка;
- в финальной архитектуре: абстракция над WebGL/WebGPU;
- UI отдельно от world rendering.

Renderer должен поддерживать:

- тайловые карты;
- чанки;
- сущности;
- слои;
- Z-levels;
- sprite atlases;
- animations;
- directional sprites;
- lighting overlays;
- particles;
- decals;
- shadows/fog overlays;
- debug rendering;
- map preview;
- editor view.

### 4.5. UI

UI должен быть browser-native.

Главная идея: не изобретать XAML/Control system как в Robust, а использовать сильные стороны браузера:

- HTML;
- CSS;
- TypeScript;
- component-based UI;
- browser layout engine;
- responsive layout;
- accessibility;
- overlays;
- drag-and-drop;
- modals;
- terminal windows;
- complex forms.

UI должен быть отделён от game simulation.

---

## 5. Репозиторий и организация

### 5.1. GitHub identity

```text
Organization display name: Open Station
Organization slug: Open-Station
Repository: honknet-engine
Full identity: Open Station / honknet-engine
```

### 5.2. Рекомендуемая структура monorepo

```text
honknet-engine/
  README.md
  LICENSE
  CONTRIBUTING.md
  CODE_OF_CONDUCT.md
  SECURITY.md
  GOVERNANCE.md
  CHANGELOG.md
  Cargo.toml
  package.json
  pnpm-workspace.yaml
  rust-toolchain.toml
  .github/
    workflows/
    ISSUE_TEMPLATE/
    PULL_REQUEST_TEMPLATE.md

  apps/
    server/
    client/
    editor/
    admin-panel/
    showcase-night-city/

  crates/
    ss15-core/
    ss15-ecs/
    ss15-world/
    ss15-map/
    ss15-physics/
    ss15-net/
    ss15-replication/
    ss15-protocol/
    ss15-persistence/
    ss15-permissions/
    ss15-modules/
    ss15-scripting/
    ss15-tools/

  packages/
    protocol-ts/
    client-core/
    renderer/
    ui/
    editor-ui/
    asset-tools/
    devtools/

  modules/
    interaction/
    inventory/
    chat/
    access/
    combat/
    medical/
    factions/
    economy/
    ai/
    vehicles/
    atmosphere/
    power/
    cyberpunk/
    space-station/

  content/
    base/
    examples/
    testbed/
    showcase-night-city/

  assets/
    base/
    editor/
    dev/

  docs/
    architecture/
    design/
    gameplay-framework/
    networking/
    renderer/
    ecs/
    modules/
    editor/
    content-pipeline/
    server-hosting/
    contribution/

  tools/
    schema-gen/
    asset-packer/
    map-compiler/
    proto-validator/
    benchmark/
    migration/
```

### 5.3. Почему monorepo

На ранней и средней стадии проекта monorepo лучше, потому что:

- клиент и сервер меняются синхронно;
- протокол меняется вместе с типами;
- проще CI;
- проще contribution flow;
- проще code review;
- проще версии модулей;
- меньше overhead на публикацию пакетов;
- проще рефакторить архитектуру.

Позже отдельные пакеты могут быть вынесены в самостоятельные репозитории или публикуемые crates/npm packages.

---

## 6. Слои продукта

### 6.1. Engine Core

Нижний технический слой.

Отвечает за:

- entity identifiers;
- component storage;
- scheduler;
- resources;
- events;
- commands;
- time/tick management;
- serialization;
- deterministic update order;
- logging;
- metrics;
- permissions primitives;
- module lifecycle.

Core не знает, что такое игрок, дверь, оружие, орган, фракция или карта города.

### 6.2. World Layer

Слой игрового мира.

Отвечает за:

- карты;
- координаты;
- чанки;
- тайлы;
- зоны;
- Z-levels;
- visibility;
- transforms;
- spatial index;
- collision world;
- entity placement;
- map streaming;
- persistence boundaries.

### 6.3. Networking Layer

Слой сетевого взаимодействия.

Отвечает за:

- подключение клиентов;
- авторизацию сессий;
- handshake;
- протокол;
- input packets;
- state snapshots;
- delta replication;
- interest management;
- reconnect;
- rate limiting;
- anti-spam;
- disconnect handling;
- transport abstraction.

### 6.4. Game Framework Layer

Базовый набор универсальных игровых систем.

Содержит системы, которые нужны большинству игр такого жанра:

- interactions;
- inventory;
- hands;
- containers;
- equipment;
- chat;
- examine;
- access checks;
- damage framework;
- status effects;
- factions;
- permissions;
- admin tools;
- prototype loading;
- UI RPC.

Это ещё не конкретная игра, но уже удобный framework для создания игры.

### 6.5. Modules Layer

Подключаемые игровые модули.

Каждый модуль должен быть отключаемым, заменяемым и расширяемым.

Модуль не должен ломать Core и не должен требовать хардкода в Core.

### 6.6. Game Layer

Конкретная игровая сборка.

Например:

```text
Night City 2045 Web
```

Она подключает нужные модули, добавляет свой контент, карты, баланс, фракции, UI-стиль, ассеты, правила, админские команды, серверную конфигурацию.

---

## 7. Серверная архитектура

### 7.1. Основная структура server runtime

```text
ServerRuntime
  ├── Config
  ├── Logger
  ├── Metrics
  ├── ModuleRegistry
  ├── AssetRegistry
  ├── PrototypeDatabase
  ├── WorldManager
  ├── TickScheduler
  ├── NetworkServer
  ├── ReplicationServer
  ├── PersistenceService
  ├── PermissionService
  ├── AdminService
  └── ConsoleService
```

### 7.2. Tick model

Сервер работает в фиксированных simulation ticks.

Рекомендуемая целевая модель:

```text
Simulation tick: 30 или 60 Hz
Network send rate: отдельно настраиваемый
Slow systems: отдельные расписания
Background jobs: async/worker pools
```

Не все системы должны обновляться каждый тик.

Пример расписания:

```text
Every tick:
- input processing
- movement
- collision
- interaction confirmations
- replication dirty flags

10 times/sec:
- AI lightweight update
- visibility recalculation for moving clients
- status effects

1 time/sec:
- hunger/thirst modules
- economy ticks
- ambient systems
- persistence checkpoints

On demand:
- map chunk loading
- asset manifest updates
- pathfinding jobs
- expensive spatial queries
```

### 7.3. ECS модель

Entity — это ID.

Component — данные без логики.

System — логика, которая работает над наборами компонентов.

Resource — глобальное состояние системы.

Event — сообщение между системами внутри одного или нескольких ticks.

Command buffer — безопасный способ отложенного изменения мира.

Пример базовых компонентов:

```text
NetEntity
Transform
MapPosition
Velocity
SpriteRef
Collider
PhysicsBody
PlayerControlled
ClientSessionRef
Container
Inventory
Interactable
Inspectable
Name
Description
DirtyReplicated
PersistenceMarker
```

### 7.4. Scheduler

Системы должны быть организованы в фазы:

```text
PreTick
  - receive network messages
  - validate sessions
  - collect inputs

Input
  - apply movement intents
  - apply interaction intents
  - apply UI actions

Simulation
  - movement
  - collisions
  - combat
  - status effects
  - AI
  - world logic

PostSimulation
  - resolve command buffers
  - update spatial index
  - mark dirty entities

Replication
  - compute visibility
  - build deltas
  - send snapshots

Persistence
  - scheduled saves
  - event log flush
  - metrics
```

### 7.5. Многопоточность

Rust-сервер должен использовать многопоточность аккуратно.

Не все системы надо параллелить сразу. Параллелизм должен применяться там, где он реально нужен:

- pathfinding;
- visibility/PVS;
- map chunk processing;
- physics broadphase;
- atmos/liquid simulation;
- asset preprocessing;
- persistence compression;
- NPC planning;
- large spatial queries.

Критическая логика должна оставаться предсказуемой и отлаживаемой.

### 7.6. Server authority boundaries

Сервер подтверждает любое действие, влияющее на игровой мир:

- movement final position;
- item pickup;
- item drop;
- attack;
- reload;
- equip;
- open door;
- use object;
- trade;
- chat visibility;
- admin command;
- damage;
- death;
- construction;
- deletion;
- spawn;
- save/load.

---

## 8. Клиентская архитектура

### 8.1. Главные обязанности клиента

Клиент отвечает за:

- рендер мира;
- рендер UI;
- ввод;
- звук;
- asset loading;
- asset cache;
- interpolation;
- client-side prediction;
- reconciliation;
- local visual effects;
- debug overlays;
- editor mode;
- UI RPC отображение.

Клиент не должен быть источником правды.

### 8.2. Client runtime

```text
ClientRuntime
  ├── Config
  ├── NetworkClient
  ├── ProtocolCodec
  ├── AssetManager
  ├── WorldView
  ├── EntityViewRegistry
  ├── Renderer
  ├── UIManager
  ├── InputManager
  ├── PredictionManager
  ├── InterpolationManager
  ├── AudioManager
  ├── DevTools
  └── LocalSettings
```

### 8.3. Rendering model

Клиент получает от сервера только необходимые данные:

- видимые чанки;
- видимые entities;
- обновления спрайтов;
- позиции;
- анимационные состояния;
- overlay states;
- UI state.

Клиент хранит локальную view-модель мира, но она является отображением серверного состояния.

### 8.4. UI model

UI должен быть построен как отдельный слой.

Типы UI:

- HUD;
- чат;
- inventory;
- character window;
- examine window;
- context menu;
- admin panel;
- debug panel;
- terminals;
- PDA-like apps;
- map apps;
- crafting UI;
- faction UI;
- market UI;
- editor panels.

UI может быть server-driven или client-local.

Server-driven UI используется для:

- inventory;
- terminals;
- admin panels;
- shops;
- character management;
- access management;
- machine interfaces.

Client-local UI используется для:

- settings;
- keybinds;
- graphics options;
- local chat filters;
- debug overlays;
- theme selection.

### 8.5. Prediction and reconciliation

Клиент может предсказывать:

- собственное движение;
- локальные анимации;
- поворот персонажа;
- открытие локального context menu;
- временную подсветку цели;
- pending interaction feedback.

Сервер подтверждает итог.

Если клиент ошибся, применяется reconciliation:

```text
Client predicts position
↓
Server sends authoritative position
↓
Client compares state
↓
If mismatch small: smooth correction
If mismatch large: snap or fast correction
```

---

## 9. Сетевой протокол

### 9.1. Schema-first protocol

Поскольку сервер на Rust, а клиент на TypeScript, протокол должен быть schema-first.

Нельзя вручную держать типы отдельно в двух языках без генерации.

Нужна единая схема:

```text
protocol/
  messages.schema
  components.schema
  enums.schema
  ui.schema
  admin.schema
```

Из неё генерируются:

```text
Rust types
TypeScript types
Protocol docs
Binary codecs
Validation tests
```

### 9.2. Категории сообщений

```text
Connection
  - Hello
  - ServerInfo
  - AuthRequest
  - AuthResult
  - JoinRequest
  - JoinResult
  - DisconnectReason

Input
  - MoveInput
  - LookInput
  - InteractInput
  - UseItemInput
  - HotkeyInput
  - UIActionInput

World State
  - FullSnapshot
  - DeltaSnapshot
  - EntityCreate
  - EntityUpdate
  - EntityDelete
  - ComponentUpdate
  - ChunkData
  - TileUpdate

Chat
  - ChatSend
  - ChatMessage
  - ChatChannelState
  - TypingState

UI
  - OpenWindow
  - CloseWindow
  - UpdateWindowState
  - UIEvent
  - UICommand

Assets
  - AssetManifest
  - AssetRequest
  - AssetChunk
  - AssetVersionMismatch

Admin
  - AdminCommand
  - AdminLogEvent
  - AdminTeleport
  - AdminSpawn
  - AdminObserve

Debug
  - Ping
  - Pong
  - MetricsUpdate
  - DebugOverlayState
```

### 9.3. Transport channels

Финальная модель с WebTransport:

```text
Reliable streams:
- auth
- chat
- UI RPC
- admin commands
- asset manifest
- inventory actions
- important game events

Unreliable datagrams:
- input frames
- movement snapshots
- frequent entity transform updates
- non-critical visual states
```

Fallback WebSocket:

```text
Single reliable bidirectional channel
- all messages multiplexed by message type
- priority queues inside client/server
- snapshot coalescing to avoid backlog
```

### 9.4. Delta replication

Сервер не должен отправлять весь мир каждый тик.

Для каждого клиента считается:

- какие entities он должен видеть;
- какие components этих entities изменились;
- какие чанки стали видимыми;
- какие чанки больше не нужны;
- какие UI states изменились.

Сервер отправляет:

```text
Create entity
Update component fields
Delete entity from client view
Tile changes
Chunk stream
UI state diff
```

### 9.5. Interest management / PVS

PVS — Potential Visibility Set.

Сервер должен понимать, какие части мира релевантны конкретному клиенту.

Факторы видимости:

- позиция игрока;
- Z-level;
- стены;
- двери;
- камеры;
- прямое зрение;
- hearing range;
- admin observe mode;
- remote camera view;
- drones;
- UI-linked remote view;
- faction sensors;
- map devices.

Interest management должен быть расширяемым.

Например, обычный игрок видит вокруг себя, а оператор дронов может видеть через камеру дрона. Админ может видеть всю карту. Камера безопасности может давать удалённый PVS.

---

## 10. Мир, карты, координаты

### 10.1. Требования к миру

Мир должен поддерживать:

- тайловые карты;
- много карт одновременно;
- Z-levels;
- многоэтажные здания;
- подземные уровни;
- интерьеры;
- экстерьеры;
- стриминг чанков;
- сохранение состояния;
- процедурные или вручную созданные области;
- зоны города;
- редакторские слои;
- динамические изменения тайлов.

### 10.2. Координатная модель

Базовая координата должна учитывать:

```text
MapId
ZLevel
X
Y
SubTile / local offset
```

Пример:

```text
WorldPosition {
  map_id: MapId,
  z: i32,
  x: i32,
  y: i32,
  local_x: f32,
  local_y: f32
}
```

### 10.3. Tile vs Entity

Tile — часть карты.

Entity — объект мира.

Примеры tile:

- пол;
- стена как базовая геометрия;
- вода;
- дорога;
- земля;
- крыша;
- технический слой.

Примеры entity:

- игрок;
- NPC;
- дверь;
- предмет;
- шкаф;
- оружие;
- машина;
- терминал;
- камера;
- контейнер;
- источник света.

Важно: движок должен позволять делать стены как tile, как entity или комбинированно, в зависимости от игры.

### 10.4. Chunks

Карта делится на чанки.

Чанк нужен для:

- сетевого стриминга;
- сохранения;
- spatial index;
- map editor performance;
- lighting update regions;
- pathfinding regions;
- visibility regions.

Пример размера:

```text
16x16 tiles
32x32 tiles
```

Размер должен быть конфигурируемым.

### 10.5. Zones

Zone — логическая область карты.

Примеры:

- район города;
- здание;
- этаж;
- комната;
- приватная территория;
- опасная зона;
- faction-controlled territory;
- no-build zone;
- interior volume;
- combat restricted area.

Zone должна поддерживать разные формы:

- polygon;
- rectangle;
- circle;
- tile mask;
- volume;
- polyline corridor.

Zone используется системами:

- чат;
- полиция/закон;
- NPC spawning;
- ambient simulation;
- экономика;
- события;
- музыка;
- lighting mood;
- admin logs;
- permissions;
- crime system;
- map labels.

---

## 11. Система прототипов и контента

### 11.1. Назначение

Prototype system нужна, чтобы контент создавался данными, а не хардкодом.

Прототип описывает сущность, тайл, item, UI, module config, zone, faction, role, effect и т.д.

### 11.2. Формат

На старте допустимы:

- YAML;
- JSON;
- RON;
- TOML.

Для проекта лучше выбрать один основной data format и не смешивать без необходимости.

Рекомендуемая модель:

```text
Human-authored content: YAML or RON
Generated/compiled content: binary cache
Runtime protocol: binary schema
```

### 11.3. Пример прототипа entity

```yaml
id: base_door
type: entity
components:
  Transform: {}
  Sprite:
    texture: "doors/basic.png"
    state: "closed"
  Collider:
    shape: "tile"
    solid: true
  Door:
    openTime: 0.4
    closeTime: 0.4
  Interactable:
    actions:
      - open_close
```

### 11.4. Наследование прототипов

Система должна поддерживать:

- inheritance;
- composition;
- overrides;
- abstract prototypes;
- validation;
- references;
- dependency checks.

Пример:

```yaml
id: reinforced_door
parent: base_door
components:
  Sprite:
    texture: "doors/reinforced.png"
  Door:
    openTime: 0.8
  AccessLocked:
    accessTags:
      - security
```

### 11.5. Валидация

Prototype validator должен проверять:

- отсутствующие assets;
- неверные component fields;
- циклическое наследование;
- неправильные enum values;
- ссылки на несуществующие prototypes;
- конфликтующие компоненты;
- deprecated fields;
- несовместимость с модулем;
- missing localization keys.

---

## 12. Модульная система

### 12.1. Что такое модуль

Модуль — это пакет, который добавляет функциональность в движок или игру.

Модуль может содержать:

```text
module.toml
server systems
client systems
components
protocol messages
prototype schemas
prototypes
assets
UI panels
editor panels
documentation
tests
migration scripts
```

### 12.2. Module manifest

Пример:

```toml
id = "open_station.inventory"
name = "Inventory"
version = "1.0.0"
side = "shared"

[dependencies]
"open_station.interaction" = ">=1.0.0"

[server]
systems = [
  "InventorySystem",
  "ContainerSystem"
]

[client]
ui = [
  "InventoryWindow"
]

[content]
prototypes = [
  "prototypes/inventory.yml"
]
```

### 12.3. Module lifecycle

Модуль проходит стадии:

```text
Discover
Validate
Load schemas
Load prototypes
Register components
Register systems
Register UI
Register commands
Start
Tick
Stop
Unload
```

### 12.4. Совместимость модулей

Движок должен уметь определять:

- required dependencies;
- optional dependencies;
- incompatible modules;
- version mismatch;
- missing protocol schema;
- server/client mismatch.

---

## 13. Базовые игровые системы конечного продукта

Этот раздел описывает не MVP, а полноценный набор систем, который должен быть возможен и поддержан архитектурой.

### 13.1. Interaction system

Система взаимодействий должна поддерживать:

- examine;
- use;
- alternate use;
- use in hand;
- use on target;
- drag/drop;
- context actions;
- range checks;
- line of sight checks;
- access checks;
- cooldowns;
- interruptible actions;
- progress bars;
- server validation;
- UI action menus.

### 13.2. Inventory system

Должна поддерживать:

- руки;
- слоты экипировки;
- контейнеры;
- сумки;
- nested containers;
- weight/volume;
- stackable items;
- item metadata;
- item condition;
- hotbar;
- drag-and-drop UI;
- server-side validation;
- anti-dupe protections.

### 13.3. Chat system

Должна поддерживать:

- local speech;
- whisper;
- shout;
- radio;
- faction channels;
- admin channels;
- emotes;
- OOC/LOOC по конфигурации;
- language filters;
- visibility by distance;
- hearing through walls по модулю;
- logging;
- moderation tools;
- rate limits;
- chat bubbles;
- optional translation module.

### 13.4. Character system

Должна поддерживать:

- аккаунт;
- персонажи;
- внешность;
- имя;
- описание;
- background;
- role eligibility;
- metadata;
- preferences;
- saved loadouts;
- character persistence;
- server rules hooks.

### 13.5. Health/Damage framework

Core не должен навязывать конкретную медицину, но framework должен позволять сложные модели:

- abstract health;
- limb health;
- organ health;
- wounds;
- bleeding;
- pain;
- unconsciousness;
- stamina;
- shock;
- armor penetration;
- damage types;
- status effects;
- healing systems;
- death states;
- body inspection UI.

Конкретная реализация должна быть модулем.

### 13.6. Combat framework

Должен поддерживать:

- melee;
- ranged;
- projectiles;
- hitscan по модулю;
- armor checks;
- penetration;
- blunt trauma;
- recoil;
- reload;
- ammo;
- weapon condition;
- cover;
- friendly fire rules;
- combat logs;
- admin audit.

### 13.7. Access/permissions in game

Система доступа должна поддерживать:

- access tags;
- ID/cards;
- biometric access по модулю;
- faction access;
- door permissions;
- terminal permissions;
- container locks;
- temporary access;
- admin overrides.

### 13.8. Faction system

Должна поддерживать:

- factions;
- subfactions;
- reputation;
- hostility;
- shared access;
- faction chat;
- faction-owned zones;
- faction jobs/roles;
- faction economy;
- faction NPCs;
- faction laws/rules.

### 13.9. Economy system

Должна быть модульной.

Возможности:

- money accounts;
- physical currency по модулю;
- bank accounts;
- transactions;
- vendors;
- shops;
- prices;
- item ownership;
- contracts;
- salaries;
- fines;
- black market;
- audit logs.

### 13.10. AI/NPC system

Должна поддерживать:

- ambient NPC;
- persistent named NPC;
- faction NPC;
- vendors;
- guards;
- patrols;
- schedules;
- memory/state;
- death persistence;
- no duplicate unique actors;
- spawn logic based on zones and context;
- pathfinding;
- behavior trees or utility AI;
- server performance budgets.

### 13.11. Vehicle system

Должна поддерживать:

- vehicles as entities;
- enter/exit;
- seats;
- ownership;
- collision;
- speed;
- damage;
- storage;
- access locks;
- server-authoritative movement;
- streaming interest;
- map restrictions.

### 13.12. Construction/destruction system

Должна поддерживать:

- building;
- deconstruction;
- repair;
- tile changes;
- entity construction;
- material costs;
- tools;
- permissions;
- structural rules;
- map persistence;
- admin rollback.

### 13.13. Atmosphere/Liquids/Power as optional modules

Атмосфера, жидкости и электричество не являются обязательным Core.

Они должны быть модулями.

Это позволяет делать:

- игру без атмосферы;
- киберпанк-город без обязательной разгерметизации;
- космическую станцию с полноценной атмосферой;
- фэнтези-игру с огнём/дымом/водой;
- упрощённые или сложные версии систем.

---

## 14. Persistent world

### 14.1. Цель

Система должна поддерживать долгоживущие миры, где состояние сохраняется между рестартами.

Сохраняться должны:

- карты;
- изменённые тайлы;
- entities;
- контейнеры;
- инвентари;
- персонажи;
- деньги;
- faction state;
- NPC state;
- world events;
- property ownership;
- doors/locks;
- damage/debris по настройке;
- logs.

### 14.2. Persistence strategy

Система должна разделять:

```text
Static content
- prototypes
- base maps
- base assets

Runtime state
- changed tiles
- spawned entities
- inventory contents
- character state
- NPC state
- economy state

Logs/events
- admin logs
- combat logs
- transactions
- world event log
```

### 14.3. Save model

Рекомендуемая модель:

- periodic checkpoints;
- event log for critical changes;
- chunk-level saves;
- entity-level dirty tracking;
- transaction boundaries for important actions;
- crash recovery;
- backup rotation;
- migration scripts.

### 14.4. Database

Подходящий вариант:

```text
PostgreSQL for accounts, characters, permissions, logs, economy
File/chunk storage for maps and world state
Object storage/CDN for assets
```

На ранней стадии можно использовать SQLite/dev storage, но конечная архитектура должна учитывать production-hosting.

---

## 15. Asset pipeline

### 15.1. Типы ассетов

Поддерживаемые ассеты:

- sprites;
- tile textures;
- sprite sheets;
- texture atlases;
- animations;
- icons;
- UI images;
- fonts;
- sounds;
- music;
- shaders;
- particle configs;
- map thumbnails;
- editor metadata.

### 15.2. Asset manifest

Клиент должен получать asset manifest:

```json
{
  "version": "hash",
  "assets": [
    {
      "id": "sprite.door.basic",
      "path": "atlas/world_01.webp",
      "hash": "...",
      "type": "sprite",
      "frames": "..."
    }
  ]
}
```

### 15.3. Caching

Браузер должен кэшировать ассеты:

- HTTP cache;
- IndexedDB;
- service worker по необходимости;
- versioned manifests;
- hash-based invalidation.

### 15.4. Asset compiler

Asset compiler должен:

- собирать atlases;
- валидировать ссылки;
- генерировать metadata;
- оптимизировать WebP/PNG;
- проверять missing frames;
- создавать editor previews;
- генерировать hashes;
- собирать production asset bundles.

---

## 16. Map editor

### 16.1. Назначение

Редактор карт должен быть частью конечного продукта.

Он может быть:

- отдельным browser app;
- встроенным dev mode в клиент;
- отдельным desktop-like web tool.

### 16.2. Возможности редактора

Редактор должен поддерживать:

- создание карты;
- слои;
- тайлы;
- entities;
- зоны;
- полигоны;
- collision preview;
- lighting preview;
- entity placement;
- prototype browser;
- search;
- undo/redo;
- multi-select;
- copy/paste;
- prefab placement;
- validation;
- export;
- import;
- diff view;
- map screenshots/thumbnails;
- live server preview по dev mode.

### 16.3. Map validation

Редактор должен проверять:

- отсутствующие prototypes;
- invalid tiles;
- blocked spawns;
- disconnected areas;
- missing zone metadata;
- overlapping zones по правилам;
- invalid access config;
- missing lighting data;
- pathfinding issues;
- performance warnings.

---

## 17. Admin tools

### 17.1. Цель

Админка должна быть встроенной частью платформы.

Игры такого типа невозможно нормально поддерживать без сильных админских инструментов.

### 17.2. Возможности

Админка должна поддерживать:

- player list;
- observe mode;
- teleport;
- spawn entity;
- delete entity;
- edit components;
- view logs;
- chat moderation;
- bans;
- role bans;
- notes;
- warnings;
- ghost/observer tools;
- permission groups;
- server console;
- map events;
- replay viewer;
- entity search;
- player inventory inspect;
- damage logs;
- transaction logs;
- rollback tools;
- event scheduler.

### 17.3. Admin permissions

Права должны быть granular:

```text
admin.observe
admin.teleport
admin.spawn
admin.delete
admin.edit_component
admin.view_logs
admin.ban
admin.roleban
admin.note
admin.server_console
admin.map_edit
admin.economy_edit
admin.replay
```

---

## 18. Security and anti-cheat

### 18.1. Основной принцип

Клиент считается недоверенным.

Любой пакет от клиента должен проходить:

- validation;
- permissions check;
- rate limit;
- state check;
- range check;
- cooldown check;
- server authority check.

### 18.2. Основные угрозы

- speedhack;
- teleport packets;
- invalid item actions;
- forged UI actions;
- chat spam;
- packet flooding;
- entity ID guessing;
- inventory duplication;
- map exploit;
- admin command spoofing;
- replay attacks;
- asset tampering;
- bot clients.

### 18.3. Защита

- server-authoritative movement;
- action validation;
- sequence numbers;
- session tokens;
- rate limits;
- server-side cooldowns;
- permission checks;
- audit logs;
- transaction model for inventory/economy;
- input sanity checks;
- anti-spam;
- admin alerts;
- optional server-side replay analysis.

---

## 19. Observability

### 19.1. Metrics

Сервер должен собирать:

- tick time;
- TPS;
- player count;
- entity count;
- chunk count;
- network in/out;
- replication cost;
- PVS cost;
- physics cost;
- AI cost;
- database latency;
- memory usage;
- save duration;
- slow systems;
- packet drops;
- reconnect rate.

### 19.2. Logs

Категории логов:

- server;
- network;
- admin;
- chat;
- combat;
- economy;
- inventory;
- permissions;
- errors;
- security;
- module loading;
- persistence.

### 19.3. Debug overlays

Клиент и админка должны уметь показывать:

- FPS;
- ping;
- interpolation delay;
- visible entities;
- loaded chunks;
- collision shapes;
- zones;
- light regions;
- PVS regions;
- network snapshot size;
- prediction errors.

---

## 20. Scripting and extension strategy

### 20.1. Проблема

Open-source платформа не может требовать, чтобы каждый контент-мейкер писал Rust.

Rust хорош для ядра и тяжёлой серверной логики, но для контента нужен более доступный слой.

### 20.2. Уровни расширения

```text
Data-only content
- prototypes
- maps
- configs

Module content
- Rust systems
- TypeScript UI

Scripting layer
- optional scripting language for game logic
```

### 20.3. Возможные scripting варианты

Возможные варианты для будущего:

- Lua;
- Rhai;
- JavaScript sandbox;
- WASM plugins;
- custom DSL for simple interactions.

### 20.4. Рекомендуемая стратегия

На уровне конечного продукта поддержать:

```text
1. Data-first content for most objects.
2. Rust modules for high-performance systems.
3. Safe scripting for server operators/content creators.
4. WASM/plugin boundary for advanced third-party extensions.
```

Scripting не должен ломать server authority и безопасность.

---

## 21. UI/UX конечного продукта

### 21.1. Общая модель

UI должен быть сильной стороной web-first подхода.

Должны быть поддержаны:

- сложные окна;
- вложенные панели;
- drag-and-drop;
- resizable windows;
- terminal-style interfaces;
- PDA/apps;
- maps;
- inventory grid/list;
- chat tabs;
- admin panels;
- editor panels;
- theme system;
- localization.

### 21.2. UI themes

Платформа должна поддерживать темы.

Примеры:

```text
base
cyberpunk
space-station
fantasy
admin-dark
high-contrast
```

Game сборка может задавать свой стиль.

### 21.3. UI RPC

Сервер может открыть UI окно у клиента:

```text
Server: OpenWindow(entityId, windowType, initialState)
Client: renders window
Client: sends UIAction
Server: validates action
Server: sends UpdateWindowState
```

### 21.4. Accessibility

Так как клиент браузерный, нужно не игнорировать accessibility:

- масштаб UI;
- настройка шрифтов;
- high contrast;
- keybinds;
- reduced motion;
- colorblind-friendly modes;
- readable chat;
- screen-size responsiveness.

---

## 22. Showcase: Night City 2045 Web

### 22.1. Роль showcase

Showcase нужен для проверки движка в настоящем gameplay.

Он не должен быть единственной целью Core, но должен постоянно выявлять реальные требования.

### 22.2. Почему Night City

Киберпанковский город проверяет универсальность лучше, чем станция:

- улицы;
- здания;
- этажи;
- районы;
- фракции;
- корпорации;
- банды;
- полиция;
- квартиры;
- магазины;
- транспорт;
- NPC;
- экономика;
- импланты;
- оружие;
- закон;
- black market;
- городские события.

Если Core выдерживает такую игру, он сможет поддержать и космическую станцию как отдельный модуль.

### 22.3. Showcase как отдельный слой

```text
apps/showcase-night-city
content/showcase-night-city
modules/cyberpunk
```

Контент showcase не должен загрязнять Core.

---

## 23. Space Station module

Так как проект называется Honknet Runtime, платформа должна позволять сделать space station сборку.

Но это должен быть модуль, а не Core.

Модуль может включать:

- atmosphere;
- power grid;
- station departments;
- access levels;
- shuttle system;
- decompression;
- space tiles;
- station jobs;
- antagonist framework;
- engineering systems;
- station events.

Ключевой принцип:

```text
Space station mechanics are optional modules, not engine assumptions.
```

---

## 24. Licensing strategy

### 24.1. Engine/Core

Рекомендуемая лицензия:

```text
Apache-2.0
```

Альтернатива:

```text
MIT
```

Apache-2.0 лучше для серьёзного open-source foundation, потому что явно покрывает патентные вопросы.

### 24.2. Game content

Контент конкретных игр должен лицензироваться отдельно.

Пример:

```text
Core engine: Apache-2.0
Base modules: Apache-2.0
Example content: CC BY 4.0 or Apache-compatible
Night City specific content: separate/custom license
Original assets: separate license
Third-party compatible assets: per-asset license
```

### 24.3. Почему разделять лицензии

Чтобы:

- Core мог использоваться другими проектами;
- уникальный контент не уносили без контроля;
- участники понимали, что они контрибьютят;
- не смешивать engine code и setting IP;
- избежать конфликтов в будущем.

---

## 25. Open-source governance

### 25.1. Роли в организации

```text
Project Owner
Core Maintainers
Engine Contributors
Module Maintainers
Content Maintainers
Documentation Maintainers
Security Team
Community Moderators
```

### 25.2. Contribution process

- issue;
- design discussion для крупных изменений;
- pull request;
- CI checks;
- code review;
- documentation update;
- changelog entry;
- merge by maintainer.

### 25.3. RFC process

Крупные архитектурные изменения должны идти через RFC.

Примеры RFC:

- новый transport layer;
- замена protocol format;
- новая scripting system;
- изменения ECS scheduler;
- новая persistence model;
- breaking changes в module API.

RFC структура:

```text
Summary
Motivation
Detailed design
Alternatives
Migration
Compatibility
Risks
Decision
```

---

## 26. CI/CD

### 26.1. CI checks

CI должен проверять:

- Rust build;
- Rust tests;
- clippy;
- rustfmt;
- TypeScript build;
- TypeScript tests;
- lint;
- protocol generation;
- schema validation;
- prototype validation;
- asset manifest validation;
- docs links;
- security audit;
- benchmark smoke tests.

### 26.2. Build artifacts

CI должен собирать:

- server binary;
- web client bundle;
- editor bundle;
- asset bundles;
- protocol docs;
- docker image;
- release archive.

### 26.3. Deployment

Production deployment:

```text
Reverse proxy / CDN
Static web client hosting
Rust game server
PostgreSQL
Object storage for assets
Metrics/logging stack
Backups
```

---

## 27. Performance targets

### 27.1. Server targets

Цели конечного продукта:

- стабильный tick rate;
- graceful degradation under load;
- thousands of entities per map;
- chunk-based replication;
- controlled memory usage;
- no unbounded packet queues;
- persistence without long stalls;
- module-level performance budgets.

### 27.2. Client targets

Клиент должен:

- работать в современных браузерах;
- не требовать установки;
- эффективно кэшировать ассеты;
- держать стабильный FPS на разумных картах;
- не рендерить невидимые чанки;
- поддерживать debug tools;
- позволять графические настройки.

### 27.3. Network targets

Сеть должна:

- не отправлять лишние entities;
- использовать delta updates;
- иметь приоритеты сообщений;
- coalesce frequent updates;
- иметь fallback transport;
- поддерживать reconnect;
- быть устойчивой к packet spam.

---

## 28. Testing strategy

### 28.1. Unit tests

Для:

- ECS components;
- systems;
- protocol encoding;
- prototype validation;
- map logic;
- permissions;
- inventory transactions;
- persistence serialization.

### 28.2. Integration tests

Для:

- client/server handshake;
- movement replication;
- inventory actions;
- map streaming;
- UI RPC;
- admin commands;
- persistence restore;
- module loading.

### 28.3. Simulation tests

Headless tests:

- 10 clients;
- 50 clients;
- 100 clients;
- 500 bots;
- entity stress;
- movement spam;
- chat spam;
- inventory spam;
- chunk streaming stress;
- save/load stress.

### 28.4. Replay tests

Система replay может использоваться для:

- debugging;
- admin review;
- regression testing;
- desync investigation;
- performance analysis.

---

## 29. Реализация конечного продукта по стадиям

Это не MVP механик, а план производства полного продукта. Каждая стадия строит часть финальной платформы.

### Stage 1 — Foundation

- создать Open Station organization;
- создать `honknet-engine` monorepo;
- настроить Rust workspace;
- настроить TypeScript workspace;
- настроить CI;
- создать базовый server runtime;
- создать базовый browser client;
- создать protocol schema pipeline;
- создать документацию архитектуры.

### Stage 2 — Core world

- ECS;
- tick scheduler;
- entities/components;
- world/map model;
- coordinates;
- chunks;
- transforms;
- spatial index;
- basic collision;
- server-authoritative movement.

### Stage 3 — Networking and replication

- WebSocket transport;
- protocol codec;
- connection/session system;
- input messages;
- full snapshot;
- delta snapshot;
- interest management baseline;
- reconnect baseline;
- network metrics.

### Stage 4 — Browser renderer

- tile rendering;
- entity rendering;
- sprite atlas;
- animation states;
- chunk loading/unloading;
- interpolation;
- debug overlays;
- asset manifest;
- browser cache.

### Stage 5 — Game framework

- interaction system;
- examine;
- hands;
- inventory;
- containers;
- equipment slots;
- chat;
- access tags;
- UI RPC;
- admin command baseline.

### Stage 6 — Content pipeline

- prototype database;
- inheritance;
- validation;
- content reload in dev;
- asset compiler;
- map format;
- map compiler;
- localization keys.

### Stage 7 — Editor tools

- browser map editor;
- prototype browser;
- tile/entity placement;
- zones;
- validation;
- map export;
- live preview;
- editor documentation.

### Stage 8 — Persistence

- database integration;
- character persistence;
- entity persistence;
- chunk saves;
- event log;
- backups;
- migrations;
- crash recovery.

### Stage 9 — Advanced networking

- WebTransport transport;
- reliable streams;
- unreliable datagrams;
- priority queues;
- transport fallback;
- bandwidth profiler;
- snapshot compression.

### Stage 10 — Advanced modules

- combat;
- health framework;
- factions;
- economy;
- AI/NPC;
- vehicles;
- construction;
- optional atmosphere;
- optional power;
- optional space-station module.

### Stage 11 — Admin and operations

- full admin panel;
- logs viewer;
- replay viewer;
- permissions UI;
- ban/roleban/note system;
- server dashboard;
- metrics dashboard;
- moderation tools.

### Stage 12 — Showcase production

- Night City 2045 Web content;
- city zones;
- factions;
- NPCs;
- economy;
- combat/medical tuning;
- cyberpunk UI theme;
- persistent characters;
- server rules integration;
- public playtest;
- performance tuning from real gameplay.

---

## 30. Главные архитектурные запреты

Чтобы проект не превратился в новый жёстко захардкоженный SS14, нужно запретить следующие ошибки.

### 30.1. Нельзя класть сеттинг в Core

Плохо:

```text
Core has AtmosSystem as mandatory system
Core has StationDepartmentComponent
Core has ShuttleSystem
Core has SpaceTile as base assumption
```

Хорошо:

```text
Atmosphere module
SpaceStation module
Shuttle module
Department module
```

### 30.2. Нельзя писать движок без showcase

Плохо:

```text
2 года делаем идеальную архитектуру без игры
```

Хорошо:

```text
Core развивается через реальные нужды showcase
```

### 30.3. Нельзя доверять клиенту

Плохо:

```text
client says: I picked up item
server accepts
```

Хорошо:

```text
client requests pickup
server validates range, visibility, permissions, container state
server applies action
server replicates result
```

### 30.4. Нельзя вручную дублировать protocol types

Плохо:

```text
Rust MsgState отдельно
TypeScript MsgState отдельно
```

Хорошо:

```text
schema → Rust types + TypeScript types + codec + tests
```

### 30.5. Нельзя делать всё модулями без стабильного Core API

Модульность не должна означать хаос.

Core API должен быть маленьким, стабильным и документированным.

---

## 31. Итоговая формула проекта

```text
Open Station / honknet-engine

Browser-first open-source framework for multiplayer 2D immersive simulation games.

Rust authoritative simulation server.
TypeScript browser client.
Schema-first network protocol.
Modular ECS-based architecture.
Data-driven content pipeline.
Browser-native UI.
Persistent world support.
Map/editor tooling.
Admin/moderation tooling.
Showcase-driven development.
```

Главная идея:

```text
Honknet Runtime is not a Space Station 14 fork.
Honknet Runtime is not a space-station-only engine.
Honknet Runtime is a universal web-first platform for deep 2D multiplayer simulation games.
```

---

## 32. Recommended external technical references

Эти ссылки используются как справочные технические ориентиры для выбранного web-first направления:

- MDN WebSocket API: https://developer.mozilla.org/en-US/docs/Web/API/WebSockets_API
- MDN WebTransport API: https://developer.mozilla.org/en-US/docs/Web/API/WebTransport_API
- MDN WebTransport interface: https://developer.mozilla.org/en-US/docs/Web/API/WebTransport
- W3C WebTransport specification: https://www.w3.org/TR/webtransport/
- Bevy ECS docs: https://docs.rs/bevy_ecs/latest/bevy_ecs/
- Bevy ECS crate page: https://crates.io/crates/bevy_ecs

