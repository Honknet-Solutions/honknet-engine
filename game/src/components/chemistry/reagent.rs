use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReagentId {
    Water,
    Bicaridine,  // Heals Brute damage
    Kelotane,    // Heals Burn damage
    Dylovene,    // Heals Toxin damage
    Inaprovline, // Stabilizes critical patients
    Plasma,      // Toxic / Combustible
    Acid,        // Corrosive
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReagentVolume {
    pub id: ReagentId,
    pub volume: f32, // uL
}
