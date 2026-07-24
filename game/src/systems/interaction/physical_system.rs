use crate::components::{
    BuckleFixtureComponent, BuckledComponent, CarriedComponent, CarryingComponent, GrabComponent,
    GrabStrength, MobState, MobStateComponent, PullingComponent,
};
use honknet_core::Entity;
use honknet_ecs::World;
use honknet_physics::PhysicsWorld;

pub fn grab(world: &mut World, actor: Entity, target: Entity) -> bool {
    if actor == target
        || !world.is_alive(actor)
        || !world.is_alive(target)
        || world
            .get::<MobStateComponent>(actor)
            .is_some_and(|state| state.state != MobState::Alive)
    {
        return false;
    }
    if let Some(grab) = world.get_mut::<GrabComponent>(actor) {
        if grab.target == target {
            grab.strength = GrabStrength::Aggressive;
        } else {
            grab.target = target;
            grab.strength = GrabStrength::Passive;
        }
        true
    } else {
        world
            .insert(
                actor,
                GrabComponent {
                    target,
                    strength: GrabStrength::Passive,
                },
            )
            .is_ok()
    }
}

pub fn release_grab(world: &mut World, actor: Entity) -> bool {
    let removed = world.remove_component::<GrabComponent>(actor);
    world.remove_component::<PullingComponent>(actor);
    removed
}

pub fn start_pulling(world: &mut World, actor: Entity, target: Entity) -> bool {
    if world
        .get::<GrabComponent>(actor)
        .is_none_or(|grab| grab.target != target)
        || world
            .get::<BuckledComponent>(target)
            .is_some_and(|buckled| buckled.fixture.is_some())
    {
        return false;
    }
    if let Some(pulling) = world.get_mut::<PullingComponent>(actor) {
        pulling.target = target;
        true
    } else {
        world
            .insert(
                actor,
                PullingComponent {
                    target,
                    maximum_distance: 4.0,
                },
            )
            .is_ok()
    }
}

pub fn stop_pulling(world: &mut World, actor: Entity) -> bool {
    world.remove_component::<PullingComponent>(actor)
}

pub fn start_carrying(world: &mut World, actor: Entity, target: Entity) -> bool {
    let aggressive_grab = world
        .get::<GrabComponent>(actor)
        .is_some_and(|grab| grab.target == target && grab.strength == GrabStrength::Aggressive);
    let incapacitated = world
        .get::<MobStateComponent>(target)
        .is_some_and(|state| state.state != MobState::Alive);
    if !aggressive_grab
        || !incapacitated
        || world.contains::<CarryingComponent>(actor)
        || world.contains::<CarriedComponent>(target)
        || world
            .get::<BuckledComponent>(target)
            .is_some_and(|state| state.fixture.is_some())
    {
        return false;
    }
    world.remove_component::<PullingComponent>(actor);
    if world.insert(actor, CarryingComponent { target }).is_err() {
        return false;
    }
    if world
        .insert(target, CarriedComponent { carrier: actor })
        .is_err()
    {
        world.remove_component::<CarryingComponent>(actor);
        return false;
    }
    true
}

pub fn drop_carried(world: &mut World, actor: Entity) -> bool {
    let target = world
        .get::<CarryingComponent>(actor)
        .map(|carrying| carrying.target);
    let Some(target) = target else {
        return false;
    };
    world.remove_component::<CarryingComponent>(actor);
    world.remove_component::<CarriedComponent>(target);
    true
}

pub fn buckle(world: &mut World, subject: Entity, fixture_entity: Entity) -> bool {
    if world
        .get::<BuckledComponent>(subject)
        .is_none_or(|state| state.fixture.is_some())
    {
        return false;
    }
    let available = world
        .get::<BuckleFixtureComponent>(fixture_entity)
        .is_some_and(|fixture| fixture.occupants.len() < usize::from(fixture.capacity));
    if !available {
        return false;
    }
    world
        .get_mut::<BuckleFixtureComponent>(fixture_entity)
        .unwrap()
        .occupants
        .push(subject);
    world.get_mut::<BuckledComponent>(subject).unwrap().fixture = Some(fixture_entity);
    true
}

pub fn unbuckle(world: &mut World, subject: Entity) -> bool {
    let fixture = world
        .get_mut::<BuckledComponent>(subject)
        .and_then(|state| state.fixture.take());
    let Some(fixture) = fixture else {
        return false;
    };
    if let Some(component) = world.get_mut::<BuckleFixtureComponent>(fixture) {
        component.occupants.retain(|occupant| *occupant != subject);
    }
    true
}

pub fn physical_interaction_system(world: &mut World, physics: &mut PhysicsWorld) {
    for actor in world.query::<CarryingComponent>() {
        let Some(target) = world
            .get::<CarryingComponent>(actor)
            .map(|carrying| carrying.target)
        else {
            continue;
        };
        let Some(actor_body) = physics.bodies.get(&actor).cloned() else {
            drop_carried(world, actor);
            continue;
        };
        if !world.is_alive(target) {
            drop_carried(world, actor);
            continue;
        }
        if let Some(target_body) = physics.bodies.get_mut(&target) {
            target_body.position = actor_body.position;
            target_body.velocity = actor_body.velocity;
        }
    }

    for actor in world.query::<PullingComponent>() {
        let Some(pulling) = world.get::<PullingComponent>(actor).cloned() else {
            continue;
        };
        let (Some(actor_body), Some(target_body)) = (
            physics.bodies.get(&actor).cloned(),
            physics.bodies.get(&pulling.target).cloned(),
        ) else {
            world.remove_component::<PullingComponent>(actor);
            continue;
        };
        let offset = target_body.position - actor_body.position;
        let distance = offset.length();
        if distance > pulling.maximum_distance {
            world.remove_component::<PullingComponent>(actor);
            continue;
        }
        if distance > 1.0 {
            if let Some(body) = physics.bodies.get_mut(&pulling.target) {
                body.position = actor_body.position + offset.normalized();
                body.velocity = actor_body.velocity;
            }
        }
    }

    for subject in world.query::<BuckledComponent>() {
        let Some(fixture) = world
            .get::<BuckledComponent>(subject)
            .and_then(|state| state.fixture)
        else {
            continue;
        };
        let Some(fixture_body) = physics.bodies.get(&fixture).cloned() else {
            unbuckle(world, subject);
            continue;
        };
        if let Some(subject_body) = physics.bodies.get_mut(&subject) {
            subject_body.position = fixture_body.position;
            subject_body.velocity = fixture_body.velocity;
        }
    }
}
