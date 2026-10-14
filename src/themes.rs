//! Every level has its own theme: a stash item you pick up (and light up to cloud the cops), a
//! colour scheme, and a name. The themes run in a fixed order and then come round again.

/// The stash item of a level. It is the power-up the player collects and uses on the cops.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Theme {
    Joint,
    RedBong,
    GoldPipe,
    Acid,
    Mushrooms,
    Hookah,
    Brownies,
    Blunt,
    Peyote,
    SpaceCake,
}

impl Theme {
    pub const ALL: [Theme; 10] = [
        Theme::Joint,
        Theme::RedBong,
        Theme::GoldPipe,
        Theme::Acid,
        Theme::Mushrooms,
        Theme::Hookah,
        Theme::Brownies,
        Theme::Blunt,
        Theme::Peyote,
        Theme::SpaceCake,
    ];

    /// The theme of zero-based level `n`.
    pub fn of_level(n: usize) -> Theme {
        Theme::ALL[n % Theme::ALL.len()]
    }

    /// The name of the place, shown as the level name.
    pub fn place(self) -> &'static str {
        match self {
            Theme::Joint => "Rolling Papers",
            Theme::RedBong => "Bong Water",
            Theme::GoldPipe => "The Golden Bowl",
            Theme::Acid => "Acid Flashback",
            Theme::Mushrooms => "Shroom Grove",
            Theme::Hookah => "Hookah Lounge",
            Theme::Brownies => "Pot Brownie Bakery",
            Theme::Blunt => "Blunt Force",
            Theme::Peyote => "Peyote Desert",
            Theme::SpaceCake => "Space Cake",
        }
    }

    /// The item, as named in the HUD and the level intro ("Grab the RED BONGS").
    pub fn item(self) -> &'static str {
        match self {
            Theme::Joint => "joints",
            Theme::RedBong => "red bongs",
            Theme::GoldPipe => "gold pipes",
            Theme::Acid => "acid tabs",
            Theme::Mushrooms => "magic mushrooms",
            Theme::Hookah => "hookahs",
            Theme::Brownies => "pot brownies",
            Theme::Blunt => "blunts",
            Theme::Peyote => "peyote buttons",
            Theme::SpaceCake => "space cakes",
        }
    }

    /// A short name for the HUD.
    pub fn short(self) -> &'static str {
        match self {
            Theme::Joint => "JOINTS",
            Theme::RedBong => "BONGS",
            Theme::GoldPipe => "PIPES",
            Theme::Acid => "ACID",
            Theme::Mushrooms => "SHROOMS",
            Theme::Hookah => "HOOKAHS",
            Theme::Brownies => "BROWNIES",
            Theme::Blunt => "BLUNTS",
            Theme::Peyote => "PEYOTE",
            Theme::SpaceCake => "SPACE CAKE",
        }
    }

    /// What happens to a cop who gets a face full of it.
    pub fn effect(self) -> &'static str {
        match self {
            Theme::Joint => "Light one up to smoke out the cops.",
            Theme::RedBong => "One big hit clouds the whole block.",
            Theme::GoldPipe => "Pack a bowl, blow it at the fuzz.",
            Theme::Acid => "Cops trip out and see colours.",
            Theme::Mushrooms => "Spores send the cops on a journey.",
            Theme::Hookah => "Fruity smoke rings stop them cold.",
            Theme::Brownies => "Cops get the munchies and zone out.",
            Theme::Blunt => "Thick smoke, very slow cops.",
            Theme::Peyote => "Cops start talking to cacti.",
            Theme::SpaceCake => "Cops float off into orbit.",
        }
    }
}

/// The display name of level `n`: the theme's place, with a number once the themes come round again.
pub fn level_name(n: usize) -> String {
    let theme = Theme::of_level(n);
    let round = n / Theme::ALL.len();
    if round == 0 {
        theme.place().to_string()
    } else {
        format!("{} {}", theme.place(), round + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_levels_follow_the_stash() {
        assert_eq!(Theme::of_level(0), Theme::Joint);
        assert_eq!(Theme::of_level(1), Theme::RedBong);
        assert_eq!(Theme::of_level(2), Theme::GoldPipe);
        assert_eq!(Theme::of_level(10), Theme::Joint);
        assert_eq!(level_name(0), "Rolling Papers");
        assert_eq!(level_name(11), "Bong Water 2");
    }
}
