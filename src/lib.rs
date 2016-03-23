#![feature(const_fn)]

extern crate libc;

mod organizer;

use libc::c_char;
use libc::c_int;
use libc::strncpy;
use std::ffi::CStr;
use std::str;

use organizer::Organizer;

static ORGANIZER: Organizer = Organizer::new();

#[allow(non_snake_case)]
#[no_mangle]
#[export_name="_RVExtension"]
pub extern "system" fn RVExtension(output: *mut c_char,
                                   _output_size: c_int,
                                   function: *const c_char) {
    let c_str = unsafe {
        assert!(!function.is_null());
        CStr::from_ptr(function)
    };

    let r_str = str::from_utf8(c_str.to_bytes()).unwrap();

    let input: Vec<&str> = r_str.split(";").collect();

    let function_name = input[0];
    let function_data = input[1];

    match ORGANIZER.call(function_name, function_data) {
        Some(ret) => unsafe {
            strncpy(output, ret.as_ptr() as *const c_char, ret.len() as usize);
        },
        None => (),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::ffi::CString;

    #[test]
    fn function_echo() {
        let function = CString::new("echo;foobar").unwrap();
        let out = CString::new("").unwrap().into_raw();
        RVExtension(out,
                    4096, // game currently calls method with this value
                    function.as_ptr());

        let result = unsafe { CString::from_raw(out) };
        assert_eq!("foobar", result.into_string().unwrap())
    }
}
