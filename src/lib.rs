#![feature(proc_macro_hygiene)]

use std::{
    io::Cursor,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

use arcropolis_api::arc_callback;
use input::InputSnapshot;
use msbt::builder::MsbtBuilder;
use patch::{FunctionHookPatch, InstructionPatch};
use serde::Deserialize;
use skyline::hooks::{InlineCtx, Region};

mod input;
mod patch;

pub(crate) static ARCROPOLIS_LOADED: AtomicBool = AtomicBool::new(false);

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

#[arc_callback]
fn listen_message_load(hash: u64, data: &mut [u8]) -> Option<usize> {
    // todo replace error handling with expect
    static RECURSION_GUARD: AtomicBool = AtomicBool::new(false);
    if RECURSION_GUARD.swap(true, Ordering::SeqCst) {
        return None;
    }
    let mut buf_in = [0u8; 20_480];
    let read = arcropolis_api::load_original_file(hash, &mut buf_in).unwrap();
    RECURSION_GUARD.store(false, Ordering::SeqCst);
    println!(
        "[GSP] Loading msg file, magic {:?} {:?}",
        &buf_in[0..10],
        read
    );
    let cursor = Cursor::new(&buf_in[0..read]);
    let msbt = msbt::Msbt::from_reader(cursor).expect("msbt read");
    let builder = MsbtBuilder::from(msbt)
        .add_label("cmn_gsp_nx", encode_msbt_str("Hold \u{e0e5} to Reveal")) // Default
        .add_label("cmn_gsp_sr", encode_msbt_str("Hold \u{e0e9} to Reveal")) // Single Joycon
        .add_label(
            "cmn_gsp_gc",
            encode_msbt_str("Hold \u{e}\u{0}\u{2}\u{2}x\u{e205}\u{e}\u{0}\u{2}\u{2}d to Reveal"),
        ); // GameCube (R trigger)
    let mut cursor = Cursor::new(data);
    builder.build().write_to(&mut cursor).expect("msbt write");
    ARCROPOLIS_LOADED.store(true, Ordering::SeqCst);
    Some(cursor.position() as usize)
}

/// Encodes a string for use in Smash's MSBT files.
/// Strings are UTF16-LE encoded on Nintendo Switch.
fn encode_msbt_str(input: &str) -> Vec<u8> {
    input
        .encode_utf16()
        .chain(std::iter::once(0u16)) // Smash null-terminates strings
        .flat_map(|s| IntoIterator::into_iter(s.to_le_bytes()))
        .collect()
}

#[skyline::main(name = "ssbu_gsp_plugin")]
pub fn main() {
    println!("[GSP] Loading...");

    let arcropolis_loaded = Path::new(
        "sd:/atmosphere/contents/01006A800016E000/romfs/skyline/plugins/libarcropolis.nro",
    )
    .exists();
    if arcropolis_loaded {
        println!("[GSP] Found arcropolis");
        listen_message_load::install(smash::hash40("ui/message/msg_common.msbt"), 20_480);
    }

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
