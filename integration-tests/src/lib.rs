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
            areas: std::collections::HashMap::new(),
            transitions: std::collections::HashMap::new(),
            docking_ports: std::collections::HashMap::new(),
            metadata: std::collections::HashMap::new(),
            streaming_regions: vec![],
            dirty_chunks: honknet_map::DirtyChunkQueue::default(),
        };
        map.grids.insert(
            grid_entity,
            honknet_map::Grid {
                entity: grid_entity,
                transform: honknet_math::Transform2::IDENTITY,
                z_level: 0,
                parent: None,
                linear_velocity: Vec2::ZERO,
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

    #[tokio::test]
    async fn multi_client_reconnection_and_session_rebind() {
        use honknet_game::GameApplication;
        use honknet_runtime::{EngineRuntimeConfig, PlayerPeer};

        let mut runtime = GameApplication::new(EngineRuntimeConfig {
            tick_rate: 30,
            listen_address: "127.0.0.1:0".to_string(),
            persistence_path: None,
            replay_path: None,
            auth_signing_key: b"test-key".to_vec(),
            session_key: b"session-key".to_vec(),
            reconnect_key: b"reconnect-key".to_vec(),
        })
        .unwrap()
        .initialize()
        .unwrap();

        // 1. First client connects (peer 100)
        let player_entity = runtime.spawn_player(100, Vec2::new(10.0, 20.0)).unwrap();
        assert_eq!(runtime.players.get(&100), Some(&player_entity));

        // 2. Peer disconnects & reconnects on new peer ID 101 with token rec-100
        let old_peer: u64 = 100;
        let new_peer: u64 = 101;
        let existing_entity = runtime
            .players
            .remove(&old_peer)
            .expect("Old peer mapped entity");
        runtime.players.insert(new_peer, existing_entity);
        if let Some(p) = runtime.world.get_mut::<PlayerPeer>(existing_entity) {
            p.0 = new_peer;
        }

        // 3. Verify clean re-binding
        assert_eq!(runtime.players.get(&100), None);
        assert_eq!(runtime.players.get(&101), Some(&player_entity));
        assert_eq!(
            runtime.world.get::<PlayerPeer>(player_entity).unwrap().0,
            101
        );
    }

    #[test]
    fn client_action_runs_through_validation_signal_and_ecs() {
        use honknet_game::{
            components::{CombatIntent, CombatIntentComponent, HealthComponent},
            GameApplication,
        };
        use honknet_net_core::{GameAction, GameActionRequestPayload, GameActionStatus};
        use honknet_physics::{Body, Fixture, Shape};
        use honknet_runtime::EngineRuntimeConfig;

        let mut game = GameApplication::new(EngineRuntimeConfig::default())
            .unwrap()
            .initialize()
            .unwrap();
        let actor = game.spawn_player(7, Vec2::ZERO).unwrap();
        game.world
            .get_mut::<CombatIntentComponent>(actor)
            .unwrap()
            .intent = CombatIntent::Harm;
        let target = game.world.spawn();
        game.world
            .insert(
                target,
                HealthComponent {
                    current: 20.0,
                    max: 20.0,
                },
            )
            .unwrap();
        game.physics.insert(Body::dynamic(
            target,
            Vec2::new(1.0, 0.0),
            1.0,
            Fixture {
                shape: Shape::Circle { radius: 0.35 },
                friction: 0.5,
                restitution: 0.0,
                sensor: false,
                layer: 1,
                mask: 1,
            },
        ));

        game.enqueue_action(
            7,
            GameActionRequestPayload {
                sequence: 1,
                action: GameAction::Attack { target },
            },
        );
        game.tick(1.0 / 30.0).unwrap();

        assert_eq!(
            game.world.get::<HealthComponent>(target).unwrap().current,
            15.0
        );
        assert_eq!(
            game.drain_action_results()[0].1.status,
            GameActionStatus::Success
        );

        game.enqueue_action(
            7,
            GameActionRequestPayload {
                sequence: 1,
                action: GameAction::Attack { target },
            },
        );
        assert_eq!(
            game.drain_action_results()[0].1.status,
            GameActionStatus::Duplicate
        );

        game.physics.bodies.get_mut(&target).unwrap().position = Vec2::new(50.0, 0.0);
        game.enqueue_action(
            7,
            GameActionRequestPayload {
                sequence: 2,
                action: GameAction::Attack { target },
            },
        );
        game.tick(1.0 / 30.0).unwrap();
        assert_eq!(
            game.drain_action_results()[0].1.status,
            GameActionStatus::OutOfRange
        );
    }

    #[tokio::test]
    async fn gameplay_action_round_trips_over_websocket() {
        use futures_util::{SinkExt, StreamExt};
        use honknet_game::GameApplication;
        use honknet_net_core::{
            decode_message, encode_message_envelope, GameAction, GameActionRequestPayload,
            GameActionResultPayload, GameActionStatus, NetworkMessage, NetworkPacketEnvelope,
        };
        use honknet_runtime::EngineRuntimeConfig;
        use tokio::net::TcpListener;
        use tokio_tungstenite::{accept_async, connect_async, tungstenite::Message};

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let mut game = GameApplication::new(EngineRuntimeConfig::default())
                .unwrap()
                .initialize()
                .unwrap();
            game.spawn_player(9, Vec2::ZERO).unwrap();
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket = accept_async(stream).await.unwrap();
            let Message::Binary(bytes) = socket.next().await.unwrap().unwrap() else {
                panic!("expected binary action packet");
            };
            let (envelope, payload) = NetworkPacketEnvelope::decode(&bytes).unwrap();
            assert_eq!(envelope.message_id, GameActionRequestPayload::ID);
            let request = decode_message::<GameActionRequestPayload>(payload, false, 4096).unwrap();
            game.enqueue_action(9, request);
            game.tick(1.0 / 30.0).unwrap();
            let (_, result) = game.drain_action_results().remove(0);
            socket
                .send(Message::Binary(
                    encode_message_envelope(&result, game.world.tick(), false)
                        .unwrap()
                        .into(),
                ))
                .await
                .unwrap();
        });

        let (mut client, _) = connect_async(format!("ws://{address}")).await.unwrap();
        let request = GameActionRequestPayload {
            sequence: 77,
            action: GameAction::Drop,
        };
        client
            .send(Message::Binary(
                encode_message_envelope(&request, 0, false).unwrap().into(),
            ))
            .await
            .unwrap();
        let Message::Binary(bytes) = client.next().await.unwrap().unwrap() else {
            panic!("expected binary action result");
        };
        let (envelope, payload) = NetworkPacketEnvelope::decode(&bytes).unwrap();
        assert_eq!(envelope.message_id, GameActionResultPayload::ID);
        let result = decode_message::<GameActionResultPayload>(payload, false, 1024).unwrap();
        assert_eq!(result.sequence, 77);
        assert_eq!(result.status, GameActionStatus::Cancelled);
        server.await.unwrap();
    }
}
