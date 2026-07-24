use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasMix {
    pub oxygen: f32,
    pub nitrogen: f32,
    pub carbon_dioxide: f32,
    pub plasma: f32,
    pub temperature: f32, // Kelvin
}

impl Default for GasMix {
    fn default() -> Self {
        Self {
            oxygen: 21.0,   // 21% O2
            nitrogen: 79.0, // 79% N2
            carbon_dioxide: 0.0,
            plasma: 0.0,
            temperature: 293.15, // 20°C / Room temperature
        }
    }
}

impl GasMix {
    pub fn total_moles(&self) -> f32 {
        self.oxygen + self.nitrogen + self.carbon_dioxide + self.plasma
    }

    pub fn pressure(&self, volume: f32) -> f32 {
        // Ideal gas law: P = nRT / V (R = 8.314)
        if volume <= 0.0 {
            return 0.0;
        }
        (self.total_moles() * 8.314 * self.temperature) / volume
    }

    pub fn remove_fraction(&mut self, fraction: f32) -> GasMix {
        let fraction = fraction.clamp(0.0, 1.0);
        let removed = GasMix {
            oxygen: self.oxygen * fraction,
            nitrogen: self.nitrogen * fraction,
            carbon_dioxide: self.carbon_dioxide * fraction,
            plasma: self.plasma * fraction,
            temperature: self.temperature,
        };
        self.oxygen -= removed.oxygen;
        self.nitrogen -= removed.nitrogen;
        self.carbon_dioxide -= removed.carbon_dioxide;
        self.plasma -= removed.plasma;
        removed
    }

    pub fn merge(&mut self, incoming: GasMix) {
        let existing_moles = self.total_moles();
        let incoming_moles = incoming.total_moles();
        let total = existing_moles + incoming_moles;
        if total > 0.0 {
            self.temperature =
                (self.temperature * existing_moles + incoming.temperature * incoming_moles) / total;
        }
        self.oxygen += incoming.oxygen;
        self.nitrogen += incoming.nitrogen;
        self.carbon_dioxide += incoming.carbon_dioxide;
        self.plasma += incoming.plasma;
    }
}
