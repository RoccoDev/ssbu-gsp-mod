use skyline::nn::hid::{
    GetNpadFullKeyState, GetNpadGcState, GetNpadHandheldState, GetNpadJoyLeftState,
    GetNpadJoyRightState, GetNpadStyleSet, NpadHandheldState,
};

macro_rules! has_bit {
    ($flags:expr, $bit:expr) => {
        $flags & (1 << $bit) != 0
    };
}

#[derive(Debug)]
enum PadStyle {
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

impl InputSnapshot {
    pub unsafe fn take(controller_id: u32, style_flags: u32) -> Self {
        let style = if has_bit!(style_flags, 0) {
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
        };

        let mut state = NpadHandheldState::default();
        (match style {
            PadStyle::Handheld => GetNpadHandheldState,
            PadStyle::GameCube => GetNpadGcState,
            PadStyle::LeftJoyconOnly => GetNpadJoyLeftState,
            PadStyle::RightJoyconOnly => GetNpadJoyRightState,
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

    pub unsafe fn take_p1() -> Self {
        let (id, style) = {
            let handheld_id = 0x20;
            let handheld_style = GetNpadStyleSet(&handheld_id as *const _).flags;

            if handheld_style != 0 {
                (handheld_id, handheld_style)
            } else {
                (0..8)
                    .map(|i| (i, GetNpadStyleSet(&i as *const _).flags))
                    .filter(|&(_, style)| style != 0)
                    .next()
                    .unwrap_or((0, 0))
            }
        };
        Self::take(id, style)
    }

    pub fn is_button_down(&self) -> bool {
        let bit = match self.style {
            PadStyle::LeftJoyconOnly => 25,  // SR (Left JoyCon)
            PadStyle::RightJoyconOnly => 27, // SR (Right JoyCon)
            _ => 7,                          // R button on other controllers
        };
        has_bit!(self.buttons, bit)
    }

    pub fn get_input_display(&self) -> &'static [u8] {
        match self.style {
            PadStyle::ProController | PadStyle::Handheld | PadStyle::DualJoycon => {
                b"cmn_button_fill_nx_r\0"
            }
            PadStyle::LeftJoyconOnly | PadStyle::RightJoyconOnly => b"cmn_button_fill_nx_sr\0",
            PadStyle::GameCube => b"cmn_button_gc_r\0",
            _ => b"cmn_empty\0",
        }
    }
}
