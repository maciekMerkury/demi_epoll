mod utils;
use utils::{errno, cast_sockaddr, result_as_errno};

use crate::{
    buffer::{self as buf, Buffer}, dpoll::{Dpoll, self}, socket::Socket, wrappers::{
        demi,
        errno::{PosixError, PosixResult},
    }
};
use core::slice;
use libc::{
    AF_INET, SOCK_STREAM, epoll_event, iovec, sigset_t, size_t, sockaddr, sockaddr_in, socklen_t,
    ssize_t,
};
use std::{
    cell::RefCell,
    mem::{self, MaybeUninit},
    os::raw::{c_int, c_void},
    sync::Mutex, time::Duration,
};

type SharedBuffer<const S: bool, T> = Mutex<RefCell<Buffer<S, T>>>;

#[inline]
const fn new_buffer<const S: bool, T>() -> SharedBuffer<S, T> {
    return Mutex::new(RefCell::new(Buffer::new()));
}

static SOCKETS: SharedBuffer<true, Socket> = new_buffer();
static DPOLLS: SharedBuffer<false, Dpoll> = new_buffer();

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dpoll_socket(domain: c_int, r#type: c_int, proto: c_int) -> c_int {
    assert!(domain == AF_INET);
    assert!(r#type == SOCK_STREAM);
    let soc = match Socket::socket() {
        Ok(s) => s,
        Err(e) => return errno(e),
    };
    let idx = SOCKETS.lock().unwrap().get_mut().allocate(soc);
    return idx.into();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dpoll_bind(
    socket_fd: c_int,
    addr: *const sockaddr,
    addr_len: socklen_t,
) -> c_int {
    assert!(addr_len as usize == mem::size_of::<libc::sockaddr_in>());
    let addr = unsafe { (addr as *const sockaddr_in).as_ref() }.unwrap();

    let idx = buf::Index::from(socket_fd);

    let res = SOCKETS
        .lock()
        .unwrap()
        .get_mut()
        .get_mut(idx)
        .unwrap()
        .bind(addr);

    return result_as_errno(res);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dpoll_listen(socket_fd: c_int, backlog: c_int) -> c_int {
    let idx = buf::Index::from(socket_fd);

    let res = SOCKETS
        .lock()
        .unwrap()
        .get_mut()
        .get_mut(idx)
        .unwrap()
        .listen(backlog);

    return result_as_errno(res);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dpoll_accept(
    socket_fd: c_int,
    addr: *mut sockaddr,
    addr_len: *mut socklen_t,
) -> c_int {
    let addr = cast_sockaddr(addr, addr_len);
    let idx = buf::Index::from(socket_fd);

    let mut lock = SOCKETS.lock().unwrap();
    let sockets = lock.get_mut();
    let res = sockets.get_mut(idx).unwrap().accept(addr);
    let soc = match res {
        Ok(s) => s,
        Err(e) => return errno(e),
    };

    return sockets.allocate(soc).into();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dpoll_close(fd: c_int) -> c_int {
    let idx: buf::Index = fd.into();

    let res = if idx.is_dpoll() {
        if idx.is_socket() {
            SOCKETS.lock().unwrap().get_mut().free(idx);
            0
        } else {
            DPOLLS.lock().unwrap().get_mut().free(idx);
            0
        }
    } else {
        unsafe { libc::close(fd) }
    };

    return res;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dpoll_write(socket_fd: c_int, buf: *const c_void, len: size_t) -> ssize_t {
    let idx: buf::Index = socket_fd.into();

    let buf = unsafe { std::ptr::slice_from_raw_parts(buf as *const u8, len).as_ref() }.unwrap();
    let res = SOCKETS
        .lock()
        .unwrap()
        .get_mut()
        .get_mut(idx)
        .unwrap()
        .write(buf);
    return match res {
        Ok(len) => len.try_into().unwrap(),
        Err(e) => errno(e) as isize,
    };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dpoll_read(socket_fd: c_int, buf: *mut c_void, len: size_t) -> ssize_t {
    let idx: buf::Index = socket_fd.into();

    let buf =
        unsafe { std::ptr::slice_from_raw_parts_mut(buf as *mut MaybeUninit<u8>, len).as_mut() }
            .unwrap();
    let res = SOCKETS
        .lock()
        .unwrap()
        .get_mut()
        .get_mut(idx)
        .unwrap()
        .read(buf);
    return match res {
        Ok(len) => len.try_into().unwrap(),
        Err(e) => errno(e) as isize,
    };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dpoll_writev(
    socket_fd: c_int,
    vecs: *const iovec,
    iovec_count: c_int,
) -> ssize_t {
    let idx: buf::Index = socket_fd.into();

    let vecs =
        unsafe { std::ptr::slice_from_raw_parts(vecs, iovec_count.try_into().unwrap()).as_ref() }
            .unwrap();

    let res = SOCKETS
        .lock()
        .unwrap()
        .get_mut()
        .get_mut(idx)
        .unwrap()
        .writev(vecs);

    return match res {
        Ok(len) => len.try_into().unwrap(),
        Err(e) => errno(e) as isize,
    };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dpoll_readv(
    socket_fd: c_int,
    vecs: *mut iovec,
    iovec_count: c_int,
) -> ssize_t {
    let idx: buf::Index = socket_fd.into();

    let vecs = unsafe {
        std::ptr::slice_from_raw_parts_mut(vecs, iovec_count.try_into().unwrap()).as_mut()
    }
    .unwrap();

    let res = SOCKETS
        .lock()
        .unwrap()
        .get_mut()
        .get_mut(idx)
        .unwrap()
        .readv(vecs);

    return match res {
        Ok(len) => len.try_into().unwrap(),
        Err(e) => errno(e) as isize,
    };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dpoll_init() -> c_int {
    return unsafe { result_as_errno(demi::meta_init()) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dpoll_create(flags: c_int) -> c_int {
    let pol = match Dpoll::create(flags) {
        Ok(s) => s,
        Err(e) => return errno(e),
    };

    let idx = DPOLLS.lock().unwrap().get_mut().allocate(pol);
    return idx.into();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dpoll_ctl(
    dpollfd: c_int,
    op: c_int,
    fd: c_int,
    event: *mut epoll_event,
) -> c_int {
    let pol: buf::Index = dpollfd.into();
    let soc: buf::Index = fd.into();
    let qd = SOCKETS.lock().unwrap().borrow().get(soc).unwrap().soc.qd;

    let op = dpoll::Operation::new(soc, qd, op, unsafe {event.as_ref()}).unwrap();
    let res = DPOLLS.lock().unwrap().get_mut().get_mut(pol).unwrap().ctl(op);
    return result_as_errno(res);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dpoll_pwait(
    dpollfd: c_int,
    events: *mut epoll_event,
    events_len: c_int,
    timeout: c_int,
    sigmask: *const sigset_t,
) -> c_int {
    let pol: buf::Index = dpollfd.into();

    let evs = unsafe { std::ptr::slice_from_raw_parts_mut(events as *mut MaybeUninit<epoll_event>, events_len.try_into().unwrap()).as_mut() }.unwrap();
    let timeout = if timeout.is_negative() { None } else { Some(Duration::from_millis(timeout as u64)) };
    let sigmask = unsafe { sigmask.as_ref() };

    let res = DPOLLS.lock().unwrap().get_mut().get_mut(pol).unwrap().pwait(SOCKETS.lock().unwrap().get_mut(), evs, timeout, sigmask);
    return match res {
        Ok(count) => count.try_into().unwrap(),
        Err(err) => errno(err)
    };
}
