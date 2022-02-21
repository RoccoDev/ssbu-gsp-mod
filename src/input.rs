use std::sync::atomic::Ordering;

use skyline::nn::hid::{
    GetNpadFullKeyState, GetNpadGcState, GetNpadHandheldState, GetNpadJoyDualState,
    GetNpadJoyLeftState, GetNpadJoyRightState, GetNpadStyleSet, NpadHandheldState,
};

macro_rules! has_bit {
    ($flags:expr, $bit:expr) => {
        $flags & (1 << $bit) != 0
    };
}

#[derive(Debug)]
pub enum PadStyle {
    ProController,
    Handheld,
    DualJoycon,
    LeftJoyconOnly,
    RightJoyconOnly,
    GameCube,
    Unknown,
}

pub struct InputSnapshot {
    style: PadStyle,
    buttons: u64,
}

impl PadStyle {
    pub fn from_flags(style_flags: u32) -> Self {
        if has_bit!(style_flags, 0) {
            PadStyle::ProController
        } else if has_bit!(style_flags, 1) {
            PadStyle::Handheld
        } else if has_bit!(style_flags, 2) {
            PadStyle::DualJoycon
        } else if has_bit!(style_flags, 3) {
            PadStyle::LeftJoyconOnly
        } else if has_bit!(style_flags, 4) {
            PadStyle::RightJoyconOnly
        } else if has_bit!(style_flags, 5) {
            PadStyle::GameCube
        } else {
            PadStyle::Unknown
        }
    }

    pub fn get_input_display(self) -> &'static [u8] {
        let arc = crate::ARCROPOLIS_LOADED.load(Ordering::SeqCst);
        match self {
            PadStyle::LeftJoyconOnly | PadStyle::RightJoyconOnly => {
                if arc {
                    b"cmn_gsp_sr\0"
                } else {
                    b"cmn_button_fill_nx_sr\0"
                }
            }
            PadStyle::GameCube => {
                if arc {
                    b"cmn_gsp_gc\0"
                } else {
                    b"cmn_button_gc_r\0"
                }
            }
            _ => {
                if arc {
                    b"cmn_gsp_nx\0"
                } else {
                    b"cmn_button_fill_nx_r\0"
                }
            }
        }
    }
}

impl InputSnapshot {
    pub unsafe fn take(controller_id: u32, style_flags: u32) -> Self {
        let style = PadStyle::from_flags(style_flags);
        let mut state = NpadHandheldState::default();
        (match style {
            PadStyle::Handheld => GetNpadHandheldState,
            PadStyle::GameCube => GetNpadGcState,
            PadStyle::LeftJoyconOnly => GetNpadJoyLeftState,
            PadStyle::RightJoyconOnly => GetNpadJoyRightState,
            PadStyle::DualJoycon => GetNpadJoyDualState,
            _ => GetNpadFullKeyState,
        })(
            &mut state as *mut NpadHandheldState,
            &controller_id as *const u32,
        );

        Self {
            style,
            buttons: state.Buttons,
        }
    }

    /// Returns whether any controller is inputting the action, and the first
    /// active controller's style.
    pub unsafe fn active_inputs() -> (PadStyle, bool) {
        let (style, has_input) = IntoIterator::into_iter([0..=7, 0x20..=0x20]) // 0x20 = handheld
            .flatten()
            .map(|i| (i, GetNpadStyleSet(&i as *const _).flags))
            .filter(|&(_, style)| style != 0)
            .fold((None, false), |(style, has_input), (id, new_style)| {
                let style = match style {
                    Some(style) => style,
                    None => new_style,
                };
                (
                    Some(style),
                    has_input || Self::take(id, new_style).is_button_down(),
                )
            });
        (
            style.map(PadStyle::from_flags).unwrap_or(PadStyle::Unknown),
            has_input,
        )
    }

    pub fn is_button_down(&self) -> bool {
        let bit = match self.style {
            PadStyle::LeftJoyconOnly => 25,  // SR (Left JoyCon)
            PadStyle::RightJoyconOnly => 27, // SR (Right JoyCon)
            _ => 7,                          // R button on other controllers
        };
        has_bit!(self.buttons, bit)
    }
}
