#![feature(proc_macro_hygiene)]

use std::{convert::TryInto, num::NonZeroIsize};

use input::InputSnapshot;
use skyline::{
    hook,
    hooks::{InlineCtx, Region},
    install_hooks,
    nn::hid::{
        GetNpadFullKeyState, GetNpadGcState, GetNpadHandheldState, GetNpadStyleSet,
        NpadHandheldState, NpadStyleSet,
    },
};
use smash::app::lua_bind::ControlModule;

mod input;

#[hook(offset = 0x01a1b5fc, inline)]
fn gsp_in_online_character_select(inline_ctx: &mut InlineCtx) {
    unsafe {
        let snapshot = InputSnapshot::take_p1();
        if !snapshot.is_button_down() {
            // "R" is not pressed
            // x1 holds the translation key
            let x1 = inline_ctx.registers[1].x.as_mut();
            *x1 = snapshot.get_input_display().as_ptr() as u64;
        }
    }
}

#[skyline::main(name = "ssbu_gsp_plugin")]
pub fn main() {
    println!("[GSP] Loading...");

    install_hooks!(gsp_in_online_character_select);
    let text_ptr = unsafe { skyline::hooks::getRegionAddress(Region::Text) } as *const u8;

    // Patch strings
    unsafe {
        // the combination of these two hides the GSP dialog completely, as if it were
        // a replay.
        skyline::patching::patch_data(0x01d5a440, &0x2a_1f_03_e0_u32).unwrap(); // mov w0, wzr, set previous GSP to 0
        skyline::patching::patch_data(0x01d5a43c, &0x1e_22_03_e0_u32).unwrap(); // scvtf s0, wzr, set new GSP to 0
    }
    println!("[GSP] Loaded!");
}
