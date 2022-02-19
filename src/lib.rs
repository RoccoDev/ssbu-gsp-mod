#![feature(proc_macro_hygiene)]

use input::InputSnapshot;
use patch::{FunctionHookPatch, InstructionPatch};
use serde::Deserialize;
use skyline::hooks::{InlineCtx, Region};

mod input;
mod patch;

#[derive(Deserialize)]
struct OffsetConfig {
    #[serde(rename = "character-select")]
    character_select: FunctionHookPatch,
    #[serde(rename = "results-screen")]
    results_screen: ResultScreenPatches,
}

#[derive(Deserialize)]
struct ResultScreenPatches {
    previous: InstructionPatch,
    new: InstructionPatch,
}

unsafe extern "C" fn gsp_in_online_character_select(inline_ctx: &mut InlineCtx) {
    let (style, button_down) = InputSnapshot::active_inputs();
    if !button_down {
        // "R" is not pressed
        // x1 holds the translation key
        let x1 = inline_ctx.registers[1].x.as_mut();
        *x1 = style.get_input_display().as_ptr() as u64;
    }
}

#[skyline::main(name = "ssbu_gsp_plugin")]
pub fn main() {
    println!("[GSP] Loading...");

    let offsets = include_str!("../offsets/13.0.1.toml");
    let config = match toml::from_str::<OffsetConfig>(offsets) {
        Ok(cfg) => cfg,
        Err(e) => {
            println!("Couldn't parse offset config: {:?}", e);
            return;
        }
    };

    let text_ptr = unsafe { skyline::hooks::getRegionAddress(Region::Text) } as *const u8;

    unsafe {
        config
            .character_select
            .patch_inline(text_ptr, gsp_in_online_character_select);
        config.results_screen.new.patch(text_ptr);
        config.results_screen.previous.patch(text_ptr);
    }

    println!("[GSP] Loaded!");
}
