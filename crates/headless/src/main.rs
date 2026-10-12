//! CPU-only host: file/CLI handling and reporting, with no alternate game rules.
mod playback;

use genesis_sim_core::{events::SimEvent, runtime::Simulation, scenarios, worldgen};
use serde::Serialize;
use std::{error::Error, path::PathBuf};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
const USAGE: &str = "genesis-headless [--scenario coast|highlands|island|fixture] [--seed N] [--ticks N] [--commands FILE]\nDefaults: coast, seed 7319, 100 ticks. Fixture is authored (seed 42); --seed is not accepted for it.\nCommand file: {\"version\":1,\"commands\":[{\"tick\":1,\"action\":{\"Terraform\":{\"x\":15,\"y\":12,\"radius\":2,\"strength\":0.4}}}]}\nTicks are 1-based; equal-tick commands execute in file order. Output is a JSON metrics record, not a save.";

struct Options {
    scenario: String,
    seed: u64,
    ticks: u64,
    commands: Option<PathBuf>,
    load: Option<PathBuf>,
    save: Option<PathBuf>,
}

impl Options {
    fn parse(args: impl IntoIterator<Item = String>) -> Result<Self> {
        let mut options = Self {
            scenario: "coast".into(),
            seed: worldgen::DEFAULT_SEED,
            ticks: 100,
            commands: None,
            load: None,
            save: None,
        };
        let mut args = args.into_iter();
        let mut seen = std::collections::BTreeSet::new();
        while let Some(flag) = args.next() {
            if !matches!(
                flag.as_str(),
                "--scenario" | "--seed" | "--ticks" | "--commands" | "--load" | "--save"
            ) {
                return Err(format!("unknown option {flag}\n{USAGE}").into());
            }
            if !seen.insert(flag.clone()) {
                return Err(format!("duplicate option {flag}").into());
            }
            let value = args.next().ok_or_else(|| format!("{flag} needs a value"))?;
            match flag.as_str() {
                "--scenario" => options.scenario = value,
                "--seed" => options.seed = value.parse()?,
                "--ticks" => options.ticks = value.parse()?,
                "--commands" => options.commands = Some(value.into()),
                "--load" => options.load = Some(value.into()),
                "--save" => options.save = Some(value.into()),
                _ => unreachable!(),
            }
        }
        if !matches!(
            options.scenario.as_str(),
            "coast" | "highlands" | "island" | "fixture"
        ) {
            return Err("--scenario must be coast, highlands, island, or fixture".into());
        }
        if options.scenario == "fixture" && seen.contains("--seed") {
            return Err(
                "fixture is authored with seed 42; --seed requires a generated scenario".into(),
            );
        }
        if options.load.is_some() && (seen.contains("--scenario") || seen.contains("--seed")) {
            return Err("--load cannot be combined with --scenario or --seed".into());
        }
        Ok(options)
    }
}

#[derive(Serialize)]
struct Metrics<'a> {
    format_version: u32,
    scenario: &'a str,
    seed: u64,
    generator_version: u32,
    completed_ticks: u64,
    simulation_time: f64,
    commands: usize,
    events: u64,
    rejected_commands: u64,
    cells: usize,
    buildings: usize,
    paths: usize,
    wells: usize,
    aquifer_stored: f64,
    well_recovered: f64,
    research_points: f32,
    stability_score: f32,
}

fn run(options: Options) -> Result<()> {
    let loaded = options
        .load
        .as_ref()
        .map(|p| genesis_storage::load(p))
        .transpose()?;
    let start_tick = loaded
        .as_ref()
        .map_or(0, |saved| saved.simulation.completed_ticks());
    let end_tick = start_tick
        .checked_add(options.ticks)
        .ok_or("tick count overflow")?;
    if end_tick >= 1_000_000_000_000 {
        return Err("run exceeds logical tick limit".into());
    }
    // Validate scheduling before generating or advancing the staged world.
    let mut playback = match &options.commands {
        Some(path) => playback::Playback::read(path, end_tick)?,
        None => playback::Playback::empty(),
    };
    playback.ensure_after(start_tick)?;
    let (mut sim, world_id, mut pending) = match loaded {
        Some(saved) => (saved.simulation, saved.world_id, saved.pending),
        None => {
            let state = match options.scenario.as_str() {
                "fixture" => scenarios::coastal_outpost(),
                "highlands" => {
                    worldgen::generate_region(options.seed, worldgen::Region::LevantHighlands)
                }
                "island" => worldgen::generate_region(options.seed, worldgen::Region::Island),
                _ => worldgen::generate(options.seed),
            };
            (
                Simulation::new(state),
                genesis_storage::new_world_id(),
                Vec::new(),
            )
        }
    };
    let commands = playback.len() + if options.ticks > 0 { pending.len() } else { 0 };
    let mut events = 0;
    let mut rejected_commands = 0;
    for tick in start_tick + 1..=end_tick {
        let mut actions = std::mem::take(&mut pending);
        actions.extend(playback.take_tick(tick));
        for event in sim.advance(&actions) {
            events += 1;
            if matches!(
                event,
                SimEvent::TerraformRejected { .. }
                    | SimEvent::BuildingRejected { .. }
                    | SimEvent::WellRejected { .. }
                    | SimEvent::PathRejected { .. }
            ) {
                rejected_commands += 1;
            }
        }
    }
    if let Some(path) = &options.save {
        let snapshot = genesis_sim_core::save::Snapshot::capture(&sim, &world_id, &pending)?;
        genesis_storage::store(path, &snapshot)?;
    }
    let metrics = Metrics {
        format_version: 1,
        scenario: if options.load.is_some() {
            "loaded"
        } else {
            &options.scenario
        },
        seed: sim.config.seed,
        generator_version: sim.geography.version,
        completed_ticks: sim.completed_ticks(),
        simulation_time: sim.time,
        commands,
        events,
        rejected_commands,
        cells: sim.grid.cells().len(),
        buildings: sim.buildings.len(),
        paths: sim.paths.len(),
        wells: sim.subsurface.wells.len(),
        aquifer_stored: sim
            .subsurface
            .columns
            .iter()
            .map(|c| f64::from(c.stored))
            .sum(),
        well_recovered: sim.subsurface.wells.iter().map(|w| w.recovered).sum(),
        research_points: sim.research_points,
        stability_score: sim.stability_score,
    };
    println!("{}", serde_json::to_string_pretty(&metrics)?);
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.as_slice() == ["--help"] || args.as_slice() == ["-h"] {
        println!("{USAGE}");
        println!(
            "Persistence: --load FILE (instead of scenario/seed), --save FILE. --ticks advances additional ticks; command ticks remain absolute. Native saves retain a previous .bak file."
        );
        return Ok(());
    }
    run(Options::parse(args)?)
}
