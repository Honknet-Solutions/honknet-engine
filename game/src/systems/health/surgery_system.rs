use crate::components::{
    BodyComponent, BodyPartComponent, SurgeryComponent, SurgeryStep, TargetZone, ToolComponent,
    WoundComponent,
};
use honknet_core::Entity;
use honknet_ecs::World;

pub fn prepare_surgery(world: &mut World, patient: Entity, zone: TargetZone, tool: Entity) -> bool {
    if !world.contains::<BodyComponent>(patient) {
        return false;
    }
    if !world.contains::<SurgeryComponent>(patient)
        && world
            .insert(
                patient,
                SurgeryComponent {
                    zone,
                    step: SurgeryStep::Incision,
                    incision_open: false,
                },
            )
            .is_err()
    {
        return false;
    }
    let Some(surgery) = world.get::<SurgeryComponent>(patient) else {
        return false;
    };
    if surgery.zone != zone || surgery.step == SurgeryStep::Complete {
        return false;
    }
    world
        .get::<ToolComponent>(tool)
        .is_some_and(|held| Some(held.tool_type) == surgery.step.required_tool())
}

pub fn complete_surgery_step(world: &mut World, patient: Entity, tool: Entity) -> bool {
    let Some(surgery) = world.get::<SurgeryComponent>(patient).cloned() else {
        return false;
    };
    if world
        .get::<ToolComponent>(tool)
        .is_none_or(|held| Some(held.tool_type) != surgery.step.required_tool())
    {
        return false;
    }
    match surgery.step {
        SurgeryStep::Incision => {
            let surgery = world.get_mut::<SurgeryComponent>(patient).unwrap();
            surgery.incision_open = true;
            surgery.step = SurgeryStep::ClampBleeders;
        }
        SurgeryStep::ClampBleeders => {
            for wound in world.query::<WoundComponent>() {
                if let Some(wound) = world
                    .get_mut::<WoundComponent>(wound)
                    .filter(|wound| wound.owner == patient)
                {
                    wound.bleeding_rate *= 0.25;
                }
            }
            world.get_mut::<SurgeryComponent>(patient).unwrap().step = SurgeryStep::Retract;
        }
        SurgeryStep::Retract => {
            world.get_mut::<SurgeryComponent>(patient).unwrap().step = SurgeryStep::Repair;
        }
        SurgeryStep::Repair => {
            let part = world
                .get::<BodyComponent>(patient)
                .and_then(|body| body.parts.get(&surgery.zone).copied());
            if let Some(part) = part.and_then(|part| world.get_mut::<BodyPartComponent>(part)) {
                part.brute_damage = (part.brute_damage - 20.0).max(0.0);
                part.burn_damage = (part.burn_damage - 10.0).max(0.0);
            }
            world.get_mut::<SurgeryComponent>(patient).unwrap().step = SurgeryStep::Close;
        }
        SurgeryStep::Close => {
            let surgery = world.get_mut::<SurgeryComponent>(patient).unwrap();
            surgery.incision_open = false;
            surgery.step = SurgeryStep::Complete;
        }
        SurgeryStep::Complete => return false,
    }
    true
}
