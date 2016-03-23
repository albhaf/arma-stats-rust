extern crate libc;

use libc::c_char;
use libc::c_int;
use libc::strncpy;
use std::ffi::CStr;
use std::str;

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

    unsafe {
        strncpy(output,
                function_name.as_ptr() as *const c_char,
                function_name.len() as usize);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::ffi::CString;

    #[test]
    fn function_echo() {
        let function = CString::new("foo;bar").unwrap();
        let out = CString::new("").unwrap().into_raw();
        RVExtension(out,
                    4096, // game currently calls method with this value
                    function.as_ptr());

        let result = unsafe { CString::from_raw(out) };
        assert_eq!("foo", result.into_string().unwrap())
    }
}
