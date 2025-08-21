use super::raw;
use std::time::Duration;

pub fn duration_to_timespec(duration: Duration) -> raw::timespec {
    raw::timespec {
        tv_sec: duration.as_secs() as libc::time_t,
        tv_nsec: duration.subsec_nanos() as libc::c_long,
    }
}

pub trait WrapperConversion<Other>: Sized
where
    Other: Sized,
{
    fn cast(self) -> Other;
}

impl WrapperConversion<libc::sockaddr_in> for raw::sockaddr_in {
    fn cast(self) -> libc::sockaddr_in {
        return unsafe { std::mem::transmute(self) };
    }
}
