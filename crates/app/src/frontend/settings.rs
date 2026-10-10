//! Small, version-tolerant preferences file. No game state is stored here.
use std::{io, path::PathBuf};

#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub scale: f32,
    pub sensitivity: f32,
    pub invert_zoom: bool,
    pub fullscreen: bool,
    pub fps: u32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            scale: 1.,
            sensitivity: 1.,
            invert_zoom: false,
            fullscreen: false,
            fps: 60,
        }
    }
}
impl Settings {
    fn path() -> Option<PathBuf> {
        let home = std::env::var_os("HOME")?;
        let base = if cfg!(target_os = "macos") {
            PathBuf::from(home).join("Library/Application Support")
        } else {
            std::env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(home).join(".config"))
        };
        Some(base.join("Genesis/settings.conf"))
    }
    pub fn load() -> Self {
        Self::path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .map_or_else(Self::default, |text| Self::parse(&text))
    }
    fn parse(text: &str) -> Self {
        let mut result = Self::default();
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match key {
                "scale" => {
                    if let Ok(v) = value.parse::<f32>()
                        && v.is_finite()
                    {
                        result.scale = v.clamp(0.85, 1.35);
                    }
                }
                "sensitivity" => {
                    if let Ok(v) = value.parse::<f32>()
                        && v.is_finite()
                    {
                        result.sensitivity = v.clamp(0.4, 2.);
                    }
                }
                "invert_zoom" => result.invert_zoom = value == "true",
                "fullscreen" => result.fullscreen = value == "true",
                "fps" => {
                    if let Ok(v @ (30 | 60 | 120)) = value.parse() {
                        result.fps = v;
                    }
                }
                _ => {}
            }
        }
        result
    }
    fn encode(&self) -> String {
        format!(
            "scale={}\nsensitivity={}\ninvert_zoom={}\nfullscreen={}\nfps={}\n",
            self.scale, self.sensitivity, self.invert_zoom, self.fullscreen, self.fps
        )
    }
    pub fn save(&self) -> io::Result<()> {
        let path =
            Self::path().ok_or_else(|| io::Error::other("No preferences directory available"))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let temporary = path.with_extension("tmp");
        std::fs::write(&temporary, self.encode())?;
        std::fs::rename(temporary, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preferences_round_trip_and_recover_from_invalid_values() {
        let settings = Settings {
            scale: 1.2,
            sensitivity: 0.6,
            invert_zoom: true,
            fullscreen: true,
            fps: 30,
        };
        assert_eq!(Settings::parse(&settings.encode()), settings);
        let parsed = Settings::parse("scale=NaN\nsensitivity=999\nfps=0\nunknown=true");
        assert_eq!(parsed.scale, 1.);
        assert_eq!(parsed.sensitivity, 2.);
        assert_eq!(parsed.fps, 60);
    }
}
