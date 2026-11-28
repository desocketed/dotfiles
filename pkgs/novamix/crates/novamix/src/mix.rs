/// One of the sinks: GAME and CHAT are on the dial, GLOBAL never is.
#[derive(Clone, Copy, Debug, Eq, PartialEq, bincode::Encode, bincode::Decode)]
pub enum Sink {
    Game,
    Chat,
    Global,
}
impl Sink {
    pub const ALL: [Sink; 3] = [Sink::Game, Sink::Chat, Sink::Global];
}
impl std::fmt::Display for Sink {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        f.write_str(match self {
            Sink::Game => "game",
            Sink::Chat => "chat",
            Sink::Global => "global",
        })
    }
}
impl std::str::FromStr for Sink {
    type Err = anyhow::Error;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text.to_lowercase().as_str() {
            "game" => Ok(Sink::Game),
            "chat" => Ok(Sink::Chat),
            "global" => Ok(Sink::Global),
            _ => Err(anyhow::anyhow!("Sink is game, chat or global, not {text}")),
        }
    }
}

/// The two sinks' volumes, each 0-100 like the base station reports them.
#[derive(Clone, Copy, Debug, Eq, PartialEq, bincode::Encode, bincode::Decode)]
pub struct Mix {
    pub game: u8,
    pub chat: u8,
}
impl Mix {
    /// Clamps each level to 100.
    pub const fn new(
        game: u8,
        chat: u8,
    ) -> Self {
        Self {
            game: if game > 100 { 100 } else { game },
            chat: if chat > 100 { 100 } else { chat },
        }
    }

    /// Both at full volume, the dial's centre.
    pub const fn full() -> Self {
        Self::new(100, 100)
    }
}
impl Default for Mix {
    fn default() -> Self {
        Self::full()
    }
}
impl std::fmt::Display for Mix {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(f, "game {}% chat {}%", self.game, self.chat)
    }
}

/// The linear gain PipeWire wants for a level in percent. Cubic, as `pactl set-sink-volume N%`
/// (and so the Python app this replaces) maps it.
pub fn gain(level: u8) -> f32 {
    let level = level.min(100) as f32 / 100.0;
    level * level * level
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_gain() {
        assert!(gain(0) == 0.0);
        assert!(gain(100) == 1.0);
        assert!((gain(50) - 0.125).abs() < 1e-6);
        assert!(gain(200) == 1.0);
    }
}
