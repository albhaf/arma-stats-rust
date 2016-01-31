extern crate libc;

use libc::c_char;
use libc::c_int;
use libc::strncpy;

#[cfg(target_os = "windows")]
#[allow(non_snake_case)]
#[no_mangle]
pub extern "stdcall" fn _RVExtension(output: *mut c_char,
                                     output_size: c_int,
                                     function: *const c_char) {
    let size = output_size as usize - 1;
    unsafe {
        strncpy(output, function, size);
    }
}
