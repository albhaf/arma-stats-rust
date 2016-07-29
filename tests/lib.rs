extern crate arma_stats;
extern crate libc;

use libc::c_char;
use libc::c_int;
use std::ffi::CString;
use std::ffi::CStr;

#[test]
fn function_echo() {
    let function = CString::new("echo;foobar").unwrap();
    let mut out = [0; 4096];

    arma_stats::RVExtension(out.as_mut_ptr() as *mut c_char,
                            out.len() as c_int,
                            function.as_ptr());

    let result = unsafe { CStr::from_ptr(out.as_ptr()) };
    assert_eq!("foobar", result.to_str().unwrap());
}

#[test]
fn function_panic() {
    let function = CString::new("panic;").unwrap();
    let mut out = [0; 4096];
    arma_stats::RVExtension(out.as_mut_ptr() as *mut c_char,
                            out.len() as c_int,
                            function.as_ptr());
}
