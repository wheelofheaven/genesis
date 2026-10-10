//! Native generated-world checks; exercises session APIs also used by the HUD.
use crate::{Result, session::Session};
use genesis_renderer::camera::Camera;
use genesis_sim_core::{actions::Action, spatial::Position};

pub fn advance(frame: u32, session: &mut Session, camera: &mut Camera) -> Result<()> {
    match frame {
        1 => {
            session.clock.paused = true;
            session.world_tools = true;
            if session.sim.buildings.len() != 6 || session.sim.geography.rivers.is_empty() {
                return Err("generated region has no settlement or drainage".into());
            }
        }
        12 => {
            let seed = session.sim.config.seed;
            let revision = session.terrain_revision;
            let old = session.sim.grid.get(192, 160).elevation;
            session.queue(Action::Terraform {
                x: 40,
                y: 40,
                radius: 1,
                strength: 1.,
            });
            session.regenerate(seed.wrapping_add(1));
            session.clock.paused = true;
            crate::terrain::home_camera(&session.sim, camera);
            if session.terrain_revision <= revision
                || session.pending_count() != 0
                || session.sim.grid.get(192, 160).elevation == old
            {
                return Err("regeneration did not replace terrain and clear stale actions".into());
            }
        }
        18 => {
            session.set_survey(true);
            session.survey.minerals = true;
            session.refresh_survey();
            let mut site = None;
            for z in 5..session.sim.grid.height - 5 {
                for x in 5..session.sim.grid.width - 5 {
                    let p = Position::new(x as f32 + 0.25, z as f32 + 0.5);
                    if session
                        .sim
                        .subsurface
                        .validate_well(
                            &session.sim.grid,
                            session.sim.sea_level,
                            &session.sim.buildings,
                            p,
                        )
                        .is_ok()
                    {
                        site = Some(p);
                        break;
                    }
                }
                if site.is_some() {
                    break;
                }
            }
            let site = site.ok_or("no usable generated aquifer")?;
            session.select(site);
            session.queue(Action::DrillWell { position: site });
            for _ in 0..100 {
                session.step_once();
            }
            if session
                .sim
                .subsurface
                .wells
                .first()
                .is_none_or(|w| w.recovered <= 0.)
            {
                return Err("generated aquifer produced no groundwater".into());
            }
        }
        26 => {
            session.survey = Default::default();
            session.world_tools = true;
            session.refresh_survey();
        }
        _ => {}
    }
    Ok(())
}
