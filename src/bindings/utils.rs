use std::mem::{self, MaybeUninit};

use libc::{c_int, sockaddr, sockaddr_in, socklen_t};
use log::trace;

use crate::wrappers::errno::{PosixError, PosixResult};

pub fn cast_sockaddr<'a>(
    addr: *mut sockaddr,
    len: *mut socklen_t,
) -> Option<&'a mut MaybeUninit<sockaddr_in>> {
    assert_eq!(addr.is_null(), len.is_null());
    if addr.is_null() {
        return None;
    }

    assert!(*unsafe { len.as_ref().unwrap() } as usize >= mem::size_of::<sockaddr_in>());

    return unsafe { (addr as *mut sockaddr_in).as_uninit_mut() };
}

pub fn errno(err: PosixError) -> c_int {
    unsafe {
        *libc::__errno_location() = err.into();
    }
    return -1;
}

/// returns 0 or -1, sets errno on error
pub fn result_as_errno(result: PosixResult<()>) -> c_int {
    trace!("result: {:?}", result);
    let error_code: c_int = match result {
        Ok(_) => return 0,
        Err(e) => e.into(),
    };

    unsafe {
        *libc::__errno_location() = error_code;
    }

    return -1;
}
