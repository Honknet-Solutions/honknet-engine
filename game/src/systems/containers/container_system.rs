use crate::components::containers::ContainerComponent;
use crate::components::hands::ItemComponent;
use honknet_core::Entity;
use honknet_ecs::World;
use tracing::info;

pub fn container_system(world: &mut World, container_entity: Entity, item: Entity) -> bool {
    if container_entity == item
        || !world.is_alive(container_entity)
        || !world.is_alive(item)
        || world
            .get::<ItemComponent>(item)
            .is_none_or(|component| component.in_container.is_some())
    {
        return false;
    }
    let item_size = world.get::<ItemComponent>(item).unwrap().size;
    let used_capacity = world
        .get::<ContainerComponent>(container_entity)
        .map(|container| {
            container
                .contents
                .iter()
                .filter_map(|entity| world.get::<ItemComponent>(*entity))
                .map(|item| item.size)
                .sum::<u32>()
        })
        .unwrap_or(u32::MAX);
    if let Some(container) = world.get_mut::<ContainerComponent>(container_entity) {
        if used_capacity.saturating_add(item_size) <= container.capacity {
            container.contents.push(item);
            world.get_mut::<ItemComponent>(item).unwrap().in_container = Some(container_entity);
            info!("Item {:?} stored in container {:?}", item, container_entity);
            return true;
        }
    }
    false
}
