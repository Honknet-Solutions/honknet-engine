use honknet_sdk::prelude::*;
pub struct ExamplePlugin;
impl honknet_sdk::GamePlugin for ExamplePlugin {
    fn name(&self) -> &'static str {
        "example"
    }
    fn startup(&mut self, world: &mut World) {
        let e = world.spawn();
        let _ = world.initialize(e);
    }
}
