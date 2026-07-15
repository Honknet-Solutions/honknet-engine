const STYLE_ID = 'honknet-hui-runtime-styles';

export function installHuiStyles(documentValue: Document): void {
  if (documentValue.getElementById(STYLE_ID)) return;
  const style = documentValue.createElement('style');
  style.id = STYLE_ID;
  style.textContent = `
    .hui-root, .hui-root * { box-sizing: border-box; }
    .hui-control { min-width: 0; min-height: 0; }
    .hui-window { display: flex; flex-direction: column; border: 1px solid rgba(120,255,205,.38); border-radius: 10px; background: rgba(7,17,24,.96); color: #e9fbff; box-shadow: 0 20px 60px rgba(0,0,0,.45); overflow: hidden; }
    .hui-window-titlebar { flex: 0 0 auto; padding: 10px 13px; border-bottom: 1px solid rgba(120,255,205,.2); background: rgba(120,255,205,.08); font-weight: 800; }
    .hui-window-content { flex: 1 1 auto; min-width: 0; min-height: 0; display: flex; flex-direction: column; }
    .hui-row { display: flex; flex-direction: row; }
    .hui-column { display: flex; flex-direction: column; }
    .hui-grid { display: grid; }
    .hui-panel { display: flex; flex-direction: column; border: 1px solid rgba(120,255,205,.18); border-radius: 7px; background: rgba(255,255,255,.025); }
    .hui-canvas, .hui-overlay { position: relative; display: block; }
    .hui-canvas > .hui-control, .hui-overlay > .hui-control { position: absolute; }
    .hui-scrollcontainer { display: flex; overflow: auto; }
    .hui-splitcontainer { display: flex; min-width: 0; min-height: 0; }
    .hui-tabcontainer { display: flex; flex-direction: column; }
    .hui-tabbar { display: flex; gap: 4px; border-bottom: 1px solid rgba(120,255,205,.18); }
    .hui-tabbutton { padding: 7px 10px; border: 0; border-bottom: 2px solid transparent; background: transparent; color: inherit; cursor: pointer; }
    .hui-tabbutton.active { border-bottom-color: #60ffc2; color: #60ffc2; }
    .hui-tabcontent { flex: 1 1 auto; min-width: 0; min-height: 0; }
    .hui-spacer { min-width: 8px; min-height: 8px; }
    .hui-label { display: block; color: inherit; }
    .hui-button { display: inline-flex; align-items: center; justify-content: center; gap: 7px; padding: 8px 12px; border: 1px solid #4ce5a1; border-radius: 6px; background: rgba(76,229,161,.11); color: #eafff7; cursor: pointer; }
    .hui-button:hover { background: rgba(76,229,161,.19); }
    .hui-button:disabled { opacity: .45; cursor: not-allowed; }
    .hui-button.pressed { background: rgba(76,229,161,.3); }
    .hui-button-icon { width: 18px; height: 18px; object-fit: contain; }
    .hui-image { display: block; max-width: 100%; max-height: 100%; }
    .hui-rsi-image { flex: 0 0 auto; background-position: 0 0; background-repeat: no-repeat; image-rendering: pixelated; }
    .hui-input, .hui-textarea, .hui-dropdown { width: 100%; padding: 8px 10px; border: 1px solid rgba(120,255,205,.28); border-radius: 5px; background: rgba(0,0,0,.32); color: #efffff; }
    .hui-checkbox { display: inline-flex; align-items: center; gap: 8px; }
    .hui-slider { width: 100%; }
    .hui-slider.vertical { writing-mode: vertical-lr; direction: rtl; }
    .hui-progress { position: relative; min-height: 18px; overflow: hidden; border: 1px solid rgba(120,255,205,.25); border-radius: 5px; background: rgba(0,0,0,.35); }
    .hui-progress-fill { height: 100%; background: linear-gradient(90deg,#35d889,#72f8bf); }
    .hui-progress-label { position: absolute; inset: 0; display: grid; place-items: center; font-size: 11px; color: #fff; }
    .hui-list { display: flex; flex-direction: column; overflow: auto; border: 1px solid rgba(120,255,205,.18); border-radius: 5px; }
    .hui-list-item { padding: 7px 9px; border: 0; border-bottom: 1px solid rgba(120,255,205,.11); background: transparent; color: inherit; text-align: left; cursor: pointer; }
    .hui-list-item.selected { background: rgba(76,229,161,.18); }
    .hui-game-view { display: grid; place-items: center; min-width: 80px; min-height: 80px; border: 1px dashed rgba(120,255,205,.3); border-radius: 7px; background: repeating-linear-gradient(45deg,rgba(120,255,205,.035),rgba(120,255,205,.035) 10px,transparent 10px,transparent 20px); color: #82f6c3; text-align: center; }
    .hui-inventory-grid { display: grid; gap: 4px; overflow: auto; }
    .hui-inventory-slot { min-width: 42px; min-height: 42px; border: 1px solid rgba(120,255,205,.22); background: rgba(0,0,0,.24); color: inherit; }
    .hui-chatbox { display: flex; flex-direction: column; min-height: 120px; }
    .hui-chat-messages { flex: 1 1 auto; overflow: auto; padding: 8px; border: 1px solid rgba(120,255,205,.18); }
    .hui-chat-input { margin-top: 6px; }
    .hui-design-node { outline-offset: 1px; }
    .hui-design-node:hover { outline: 1px solid rgba(91,224,255,.65); }
    .hui-design-node.hui-design-selected { outline: 2px solid #5be0ff; }
    .hui-design-empty::after { content: 'Drop controls here'; display: grid; place-items: center; min-height: 48px; color: rgba(160,220,235,.55); border: 1px dashed rgba(91,224,255,.28); }
  `;
  documentValue.head.append(style);
}
