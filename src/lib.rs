extern crate chrono;
extern crate libc;
extern crate reqwest;
extern crate serde_json;

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
#[export_name = "_RVExtension"]
pub extern "system" fn RVExtension(
    output: *mut c_char,
    output_size: c_int,
    function: *const c_char,
) {
    let c_str = unsafe {
        assert!(!function.is_null());
        CStr::from_ptr(function)
    };

    let input: Vec<&str> = match c_str.to_str() {
        Ok(s) => s.split(";").collect(),
        _ => return,
    };

    match panic::catch_unwind(|| {
        // Ignore poisoned mutex for now, hopefully it's not something too bad
        let mut guard = match ORGANIZER.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        guard.call(input[0], input[1].to_string())
    }) {
        Ok(Some(ret)) => unsafe {
            strncpy(output, ret.as_ptr() as *const c_char, output_size as usize);
        },
        Ok(None) => (),
        Err(e) => {
            let err: &dyn std::fmt::Debug = {
                if let Some(e) = e.downcast_ref::<String>() {
                    e
                } else if let Some(e) = e.downcast_ref::<&str>() {
                    e
                } else {
                    &e
                }
            };

            println!("error: {:?}", err);
        }
    };
}
