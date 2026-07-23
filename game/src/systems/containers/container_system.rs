use crate::components::containers::ContainerComponent;
use crate::components::hands::ItemComponent;
use honknet_core::Entity;
use honknet_ecs::World;
use tracing::info;

pub fn container_system(world: &mut World, container_entity: Entity, item: Entity) -> bool {
    if let Some(container) = world.get_mut::<ContainerComponent>(container_entity) {
        if (container.contents.len() as u32) < container.capacity {
            container.contents.push(item);
            if let Some(item_comp) = world.get_mut::<ItemComponent>(item) {
                item_comp.in_container = Some(container_entity);
            }
            info!("Item {:?} stored in container {:?}", item, container_entity);
            return true;
        }
    }
    false
}
