use std::{mem::MaybeUninit, time::Duration};

use libc::epoll_event;
use log::trace;

use crate::{
    dpoll::operation::EpollOperation,
    wrappers::errno::{PosixError, PosixResult},
};

#[repr(transparent)]
#[derive(Debug)]
pub struct Epoll {
    fd: i32,
}

impl Drop for Epoll {
    fn drop(&mut self) {
        trace!("dropping {}", self.fd);
        unsafe { libc::close(self.fd) };
    }
}

impl Epoll {
    pub fn create(flags: i32) -> PosixResult<Self> {
        let fd = unsafe { libc::epoll_create1(flags) };

        if fd.is_negative() {
            return PosixError::from_errno().map(|_| unreachable!());
        }

        trace!("new epoll: {fd}");
        return Ok(Self { fd });
    }

    pub fn ctl(&mut self, op: EpollOperation) -> PosixResult<()> {
        let EpollOperation { op, fd, event } = op;
        let res = unsafe { libc::epoll_ctl(self.fd, op, fd, event) };

        return if res.is_negative() {
            PosixError::from_errno()
        } else {
            Ok(())
        };
    }

    pub fn wait(
        &mut self,
        evs: &mut [MaybeUninit<epoll_event>],
        timeout: Option<Duration>,
    ) -> PosixResult<usize> {
        let timeout: i32 = timeout.map_or(-1, |d| d.as_millis().try_into().unwrap());
        trace!("waiting for {timeout}");
        let res = unsafe {
            libc::epoll_wait(
                self.fd,
                evs.as_mut_ptr() as *mut epoll_event,
                evs.len().try_into().unwrap(),
                timeout,
            )
        };

        return if res.is_negative() {
            PosixError::from_errno().map(|_| unreachable!())
        } else {
            Ok(res.try_into().unwrap())
        };
    }
}
