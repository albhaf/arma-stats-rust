extern crate arma_stats;

use std::ffi::CString;

#[test]
fn function_echo() {
    let function = CString::new("echo;foobar").unwrap();
    let out = CString::new("").unwrap().into_raw();
    arma_stats::RVExtension(out,
                            4096, // game currently calls method with this value
                            function.as_ptr());

    let result = unsafe { CString::from_raw(out) };
    assert_eq!("foobar", result.into_string().unwrap())
}
