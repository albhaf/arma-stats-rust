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
use std::panic;
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

    let input: Vec<&str> = match str::from_utf8(c_str.to_bytes()) {
        Ok(s) => s.split(";").collect(),
        _ => return,
    };

    let ret = match panic::catch_unwind(|| ORGANIZER.lock().unwrap().call(input[0], input[1])) {
        Ok(Some(s)) => s,
        Ok(None) => return,
        Err(_) => {
            // TODO: log error
            return;
        }
    };

    unsafe {
        strncpy(output, ret.as_ptr() as *const c_char, ret.len() as usize);
    }
}
