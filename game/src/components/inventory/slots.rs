use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EquipmentSlot {
    Head,
    Mask,
    Jumpsuit,
    OuterClothing,
    Gloves,
    Shoes,
    Belt,
    PocketLeft,
    PocketRight,
    IdCard,
}
