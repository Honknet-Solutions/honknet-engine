#[cfg(test)]
mod tests {
    use honknet_ecs::{Component, World};
    use honknet_math::Vec2;
    use honknet_physics::{Body, Fixture, PhysicsWorld, Shape};
    use honknet_testing::HeadlessHarness;

    #[derive(Debug, Clone, Copy)]
    struct Position(Vec2);
    impl Component for Position {}

    #[derive(Debug, Clone, Copy)]
    struct Velocity(Vec2);
    impl Component for Velocity {}

    #[tokio::test]
    async fn client_server_transport_round_trip() {
        assert!(
            HeadlessHarness::new()
                .round_trip(b"integration-state")
                .await
        )
    }

    #[tokio::test]
    async fn end_to_end_ecs_physics_map_integration() {
        let mut world = World::default();
        let mut physics = PhysicsWorld::default();

        let grid_entity = world.spawn();
        let mut map = honknet_map::Map {
            id: 1,
            tile_size: 1.0,
            tiles: vec![
                honknet_map::TileDef {
                    id: "space".into(),
                    solid: false,
                    friction: 0.0,
                    resource: None,
                },
                honknet_map::TileDef {
                    id: "floor".into(),
                    solid: true,
                    friction: 0.5,
                    resource: Some("floor.png".into()),
                },
            ],
            grids: std::collections::HashMap::new(),
            metadata: std::collections::HashMap::new(),
            streaming_regions: vec![],
        };
        map.grids.insert(
            grid_entity,
            honknet_map::Grid {
                entity: grid_entity,
                transform: honknet_math::Transform2::IDENTITY,
                chunks: std::collections::HashMap::new(),
                revision: 0,
            },
        );

        // 1. Tile mutation on map grid
        map.set_tile(grid_entity, 0, 0, 1).unwrap();
        assert_eq!(
            map.tile(grid_entity, 0, 0).map(|t| t.id.as_str()),
            Some("floor")
        );

        // 2. Spawn ECS Entity with components
        let player = world.spawn();
        world.insert(player, Position(Vec2::new(2.0, 3.0))).unwrap();
        world.insert(player, Velocity(Vec2::new(1.0, 0.0))).unwrap();
        world.initialize(player).unwrap();

        assert!(world.contains::<Position>(player));
        assert!(world.contains::<Velocity>(player));

        // 3. Attach Physics body to ECS Entity
        physics.insert(Body::dynamic(
            player,
            Vec2::new(2.0, 3.0),
            1.0,
            Fixture {
                shape: Shape::Circle { radius: 0.5 },
                friction: 0.2,
                restitution: 0.1,
                sensor: false,
                layer: 1,
                mask: 1,
            },
        ));

        // 4. Simulate physics step
        physics.bodies.get_mut(&player).unwrap().velocity = Vec2::new(2.0, 0.0);
        physics.step(0.1);

        // 5. Verify ECS state sync
        let updated_body = physics.bodies.get(&player).unwrap();
        world.get_mut::<Position>(player).unwrap().0 = updated_body.position;
        world.get_mut::<Velocity>(player).unwrap().0 = updated_body.velocity;

        assert!((world.get::<Position>(player).unwrap().0.x - 2.2).abs() < 0.01);

        // 6. Test component removal
        assert!(world.remove_component::<Velocity>(player));
        assert!(!world.contains::<Velocity>(player));

        // 7. Test entity despawn
        world.despawn(player).unwrap();
        assert!(!world.is_alive(player));
    }
}
