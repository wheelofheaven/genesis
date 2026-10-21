use super::super::theme::*;
use genesis_sim_core::supply_outlook::{Home, Outlook};

pub(super) fn summary(ui: &mut egui::Ui, outlook: &Outlook, food: bool) {
    let (next, short, use_name) = if food {
        (outlook.next_food_ms, outlook.food_short_homes(), "meal")
    } else {
        (outlook.next_water_ms, outlook.water_short_homes(), "drink")
    };
    let Some(ms) = next else {
        return;
    };
    ui.label(format!(
        "Next {use_name} · {:.1} logical s",
        ms as f64 / 1000.
    ))
    .on_hover_text("Simulation time: pauses with the game and follows the speed setting.");
    ui.colored_label(if short > 0 { ERROR } else { ACCENT }, if short > 0 {
        format!("{short} {} short at current stocks", if short == 1 { "home" } else { "homes" })
    } else { "Every home covers the next use".into() })
        .on_hover_text("Counts only supplies already at each home. Source stocks, reservations and carried supplies cannot be consumed until delivered. This is not a delivery-time forecast.");
}

pub(super) fn home_line(home: &Home, food: bool) -> String {
    if food {
        let short = home.food_shortfall();
        let mut text = if short > 0 {
            format!(
                "Next meal · short {short} {}",
                if short == 1 { "portion" } else { "portions" }
            )
        } else {
            "Next meal covered".into()
        };
        if home.carried_food > 0 {
            text.push_str(&format!(" · {} carried", home.carried_food));
        }
        text
    } else {
        let short = home.water_shortfall();
        let mut text = if short > 0. {
            if short < 0.01 {
                "Next drink · short <0.01 u".into()
            } else {
                format!("Next drink · short {short:.2} u")
            }
        } else {
            "Next drink covered".into()
        };
        if home.carried_water > 0. {
            text.push_str(&format!(" · {:.1} u carried", home.carried_water));
        }
        text
    }
}
