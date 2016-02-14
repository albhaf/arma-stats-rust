extern crate libc;

use libc::c_char;
use libc::c_int;
use libc::strncpy;

#[cfg(target_os = "windows")]
#[allow(non_snake_case)]
#[no_mangle]
#[export_name="_RVExtension@12"]
pub extern "stdcall" fn RVExtension(output: *mut c_char,
                                    output_size: c_int,
                                    function: *const c_char) {
    let size = output_size as usize - 1;
    unsafe {
        strncpy(output, function, size);
    }
}
