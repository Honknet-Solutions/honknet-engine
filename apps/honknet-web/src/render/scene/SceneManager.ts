import { Container, Application } from 'pixi.js';

export default class SceneManager {
    public readonly stage: Container;
    
    // World Root and its layers
    public readonly worldRoot: Container;
    public readonly backgroundLayer: Container;
    public readonly tileLayer: Container;
    public readonly underEntityEffects: Container;
    public readonly entityLayer: Container;
    public readonly overEntityEffects: Container;
    public readonly lightingLayer: Container;
    public readonly decalLayer: Container;
    public readonly worldUiLayer: Container;
    public readonly debugLayer: Container;

    // Screen Root and its layers
    public readonly screenRoot: Container;
    public readonly hudLayer: Container;
    public readonly windowLayer: Container;
    public readonly tooltipLayer: Container;
    public readonly contextMenuLayer: Container;
    public readonly modalLayer: Container;
    public readonly cursorLayer: Container;

    constructor(app: Application) {
        this.stage = app.stage;

        this.worldRoot = new Container();
        this.backgroundLayer = new Container();
        this.tileLayer = new Container();
        this.underEntityEffects = new Container();
        this.entityLayer = new Container();
        this.overEntityEffects = new Container();
        this.lightingLayer = new Container();
        this.decalLayer = new Container();
        this.worldUiLayer = new Container();
        this.debugLayer = new Container();

        this.worldRoot.addChild(this.backgroundLayer);
        this.worldRoot.addChild(this.tileLayer);
        this.worldRoot.addChild(this.underEntityEffects);
        this.worldRoot.addChild(this.entityLayer);
        this.worldRoot.addChild(this.overEntityEffects);
        this.worldRoot.addChild(this.lightingLayer);
        this.worldRoot.addChild(this.decalLayer);
        this.worldRoot.addChild(this.worldUiLayer);
        this.worldRoot.addChild(this.debugLayer);

        this.screenRoot = new Container();
        this.hudLayer = new Container();
        this.windowLayer = new Container();
        this.tooltipLayer = new Container();
        this.contextMenuLayer = new Container();
        this.modalLayer = new Container();
        this.cursorLayer = new Container();

        this.screenRoot.addChild(this.hudLayer);
        this.screenRoot.addChild(this.windowLayer);
        this.screenRoot.addChild(this.tooltipLayer);
        this.screenRoot.addChild(this.contextMenuLayer);
        this.screenRoot.addChild(this.modalLayer);
        this.screenRoot.addChild(this.cursorLayer);

        this.stage.addChild(this.worldRoot);
        this.stage.addChild(this.screenRoot);
    }
}
