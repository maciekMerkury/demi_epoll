use std::{mem::MaybeUninit, time::Duration};

use libc::{c_int, epoll_event};

use crate::wrappers::errno::{PosixError, PosixResult};

use super::operation::Operation;

#[repr(transparent)]
#[derive(Debug)]
pub struct Epoll {
    fd: i32
}

impl Drop for Epoll {
    fn drop(&mut self) {
        unsafe { libc::close(self.fd) };
    }
}

impl Epoll {
    pub fn create(flags: i32) -> PosixResult<Self> {
        let fd = unsafe {
            libc::epoll_create1(flags)
        };

        if fd.is_negative() {
            return PosixError::from_errno().map(|_| unreachable!());
        }

        return Ok(Self { fd });
    }

    pub fn ctl(&mut self, op: Operation) -> PosixResult<()> {
        let (op, fd, mut event) = op.to_raw();
        let ptr = match event.as_mut() {
            Some(r) => r,
            None => std::ptr::null_mut(),
        };
        let res = unsafe { libc::epoll_ctl(self.fd, op, fd, ptr) };

        return PosixError::from_errno();
    }

    pub fn wait(&mut self, evs: &mut [MaybeUninit<epoll_event>], timeout: Option<Duration>) -> PosixResult<usize> {

        let timeout: i32 = timeout.map_or(-1, |d| d.as_millis().try_into().unwrap());
        let res = unsafe { libc::epoll_wait(self.fd, evs.as_mut_ptr() as *mut epoll_event, evs.len().try_into().unwrap(), timeout) };

        return if (res.is_negative()) {
            PosixError::from_errno().map(|_| unreachable!())
        } else {
            Ok(res.try_into().unwrap())
        }
    }
}

