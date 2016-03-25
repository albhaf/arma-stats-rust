extern crate libc;
extern crate hyper;
extern crate serde_json;
extern crate time;

#[macro_use]
extern crate lazy_static;

mod organizer;

use libc::c_char;
use libc::c_int;
use libc::strncpy;
use std::ffi::CStr;
use std::str;
use std::sync::Mutex;

use organizer::Organizer;

lazy_static! {
    static ref ORGANIZER: Mutex<Organizer> = Mutex::new(Organizer::new());
}

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

    // TODO: make prettier and perhaps some error handling
    match ORGANIZER.lock().unwrap().call(function_name, function_data) {
        Some(ret) => unsafe {
            strncpy(output, ret.as_ptr() as *const c_char, ret.len() as usize);
        },
        None => (),
    }
}
