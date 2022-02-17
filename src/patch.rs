use std::convert::TryInto;

use serde::Deserialize;
use skyline::{
    hooks::{A64InlineHook, InlineCtx},
    libc::c_void,
};

#[derive(Deserialize)]
pub struct FunctionHookPatch {
    pub offset: usize,
}

#[derive(Deserialize)]
pub struct InstructionPatch {
    offset: usize,
    expect: [u8; 4],
    replace: [u8; 4],
}

impl FunctionHookPatch {
    pub unsafe fn patch_inline<P>(
        &self,
        text_ptr: *const P,
        callback: unsafe extern "C" fn(&mut InlineCtx),
    ) {
        A64InlineHook(
            text_ptr.offset(self.offset.try_into().unwrap()) as *const c_void,
            callback as *const c_void,
        );
    }
}

impl InstructionPatch {
    pub unsafe fn patch(&self, text_ptr: *const u8) -> bool {
        let expect = self.expect;
        let data_ptr = text_ptr.offset(self.offset.try_into().unwrap());
        if expect != *core::slice::from_raw_parts(data_ptr, expect.len()) {
            println!(
                "Couldn't patch function at offset .text+{}: old contents didn't match",
                self.offset
            );
            return false;
        };
        match skyline::patching::patch_data(self.offset, &self.replace) {
            Ok(()) => true,
            Err(e) => {
                println!(
                    "Couldn't patch function at offset .text+{}: {:?}",
                    self.offset, e
                );
                false
            }
        }
    }
}
