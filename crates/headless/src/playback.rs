//! Assigned logical boundaries, never wall-clock input arrivals.
use genesis_sim_core::actions::Action;
use serde::Deserialize;
use std::{collections::VecDeque, fs::File, io::Read, path::Path};

const MAX_BYTES: u64 = 1024 * 1024;
const MAX_COMMANDS: usize = 10_000;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Log {
    version: u32,
    commands: Vec<Scheduled>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Scheduled {
    tick: u64,
    action: Action,
}

pub(super) struct Playback {
    commands: VecDeque<Scheduled>,
}

impl Playback {
    pub fn empty() -> Self {
        Self {
            commands: VecDeque::new(),
        }
    }

    pub fn read(path: &Path, ticks: u64) -> super::Result<Self> {
        let mut bytes = Vec::new();
        File::open(path)?
            .take(MAX_BYTES + 1)
            .read_to_end(&mut bytes)?;
        Self::decode(&bytes, ticks).map_err(|e| format!("{}: {e}", path.display()).into())
    }

    fn decode(bytes: &[u8], ticks: u64) -> super::Result<Self> {
        if bytes.len() as u64 > MAX_BYTES {
            return Err("command log exceeds 1 MiB limit".into());
        }
        let log: Log = serde_json::from_slice(bytes)?;
        if log.version != 1 {
            return Err("unsupported command log version (expected 1)".into());
        }
        if log.commands.len() > MAX_COMMANDS {
            return Err("command log exceeds 10000-command limit".into());
        }
        let mut previous = 0;
        for (index, entry) in log.commands.iter().enumerate() {
            if entry.tick == 0 || entry.tick > ticks || entry.tick < previous {
                return Err(format!(
                    "command {index}: ticks must be nondecreasing and within 1..={ticks}"
                )
                .into());
            }
            previous = entry.tick;
        }
        Ok(Self {
            commands: log.commands.into(),
        })
    }

    pub fn len(&self) -> usize {
        self.commands.len()
    }

    pub fn ensure_after(&self, tick: u64) -> super::Result<()> {
        if self
            .commands
            .front()
            .is_some_and(|entry| entry.tick <= tick)
        {
            return Err("command log includes an already completed tick".into());
        }
        Ok(())
    }

    pub fn take_tick(&mut self, tick: u64) -> Vec<Action> {
        let mut actions = Vec::new();
        while self
            .commands
            .front()
            .is_some_and(|entry| entry.tick == tick)
        {
            if let Some(entry) = self.commands.pop_front() {
                actions.push(entry.action);
            }
        }
        actions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use genesis_sim_core::{events::SimEvent, runtime::Simulation, scenarios};

    #[test]
    fn recorded_boundaries_preserve_conflicts_and_commit_once() {
        let mut playback = Playback::decode(
            include_bytes!("../../sim_core/fixtures/coastal.commands.json"),
            100,
        )
        .unwrap();
        let mut sim = Simulation::new(scenarios::coastal_outpost());
        let mut placed = 0;
        let mut rejected = 0;
        for tick in 1..=100 {
            for event in sim.advance(&playback.take_tick(tick)) {
                match event {
                    SimEvent::BuildingPlaced { .. } => {
                        assert_eq!(tick, 2);
                        placed += 1;
                    }
                    SimEvent::BuildingRejected { .. } => {
                        assert_eq!(tick, 2);
                        rejected += 1;
                    }
                    SimEvent::WellDrilled { .. } => assert_eq!(tick, 3),
                    SimEvent::WellOperationChanged { .. } => assert_eq!(tick, 50),
                    _ => {}
                }
            }
            if tick == 49 {
                assert!(sim.subsurface.wells[0].active);
            }
        }
        assert_eq!((placed, rejected, playback.len()), (1, 1, 0));
        assert_eq!(sim.completed_ticks(), 100);
        assert!(sim.subsurface.wells[0].recovered > 0.);
        assert!(!sim.subsurface.wells[0].active);
    }

    #[test]
    fn invalid_logs_fail_before_advancement() {
        for log in [
            r#"{"version":2,"commands":[]}"#,
            r#"{"version":1,"commands":[],"typo":true}"#,
            r#"{"version":1,"commands":[{"tick":0,"action":{"BuildOutpost":{"x":1,"y":1}}}]}"#,
            r#"{"version":1,"commands":[{"tick":101,"action":{"BuildOutpost":{"x":1,"y":1}}}]}"#,
            r#"{"version":1,"commands":[{"tick":2,"action":{"BuildOutpost":{"x":1,"y":1}}},{"tick":1,"action":{"BuildOutpost":{"x":1,"y":1}}}]}"#,
        ] {
            assert!(Playback::decode(log.as_bytes(), 100).is_err());
        }
        assert!(Playback::decode(&vec![b' '; MAX_BYTES as usize + 1], 100).is_err());
    }
}
