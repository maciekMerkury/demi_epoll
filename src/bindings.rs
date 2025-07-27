use std::{cell::RefCell, mem::{self, MaybeUninit}, os::raw::{c_int, c_void}, sync::Mutex};
use libc::{epoll_event, iovec, sigset_t, size_t, sockaddr, sockaddr_in, socklen_t, ssize_t, AF_INET, SOCK_STREAM};
use crate::{buffer::{Buffer, self as buf}, socket::Socket, wrappers::{demi, errno::{PosixError, PosixResult}}};

type SharedBuffer<const S: bool, T> = Mutex<RefCell<Buffer<S, T>>>;

#[inline]
const fn new_buffer<const S: bool, T>() -> SharedBuffer<S, T> {
    return Mutex::new(RefCell::new(Buffer::new()));
}

fn cast_sockaddr<'a>(addr: *mut sockaddr, len: *mut socklen_t) -> Option<&'a mut MaybeUninit<sockaddr_in>> {
    assert_eq!(addr.is_null(), len.is_null());
    if addr.is_null() {
        return None;
    }

    assert!(*unsafe { len.as_ref().unwrap() } as usize >= mem::size_of::<libc::sockaddr_in>());

    return unsafe { (addr as *mut sockaddr_in).as_uninit_mut() };
}

fn errno(err: PosixError) -> c_int {
    unsafe {
        *libc::__errno_location() = err.into();
    }
    return -1;
}

/// returns 0 or -1, sets errno on error
fn result_as_errno(result: PosixResult<()>) -> c_int {
    let error_code: c_int = match result {
        Ok(_) => return 0,
        Err(e) => e.into(),
    };

    unsafe { *libc::__errno_location() = error_code; }

    return -1;
}

static SOCKETS: SharedBuffer<true, Socket> = new_buffer();

pub unsafe extern "C" fn dpoll_socket(domain: c_int, r#type: c_int, proto: c_int) -> c_int {
    assert!(domain == AF_INET );
assert!(r#type == SOCK_STREAM);
    let soc = match Socket::socket() {
        Ok(s) => s,
        Err(e) => return errno(e),
    };
    let idx = SOCKETS.lock().unwrap().get_mut().allocate(soc);
    return idx.into();
}

pub unsafe extern "C" fn dpoll_bind(socket_fd: c_int, addr: *const sockaddr, addr_len: socklen_t) -> c_int {
    assert!(addr_len as usize == mem::size_of::<libc::sockaddr_in>());
    let addr = unsafe { (addr as *const sockaddr_in).as_ref() }.unwrap();

    let idx = buf::Index::from(socket_fd);
    assert!(idx.is_dpoll() );
assert!(idx.is_socket());

    let res = SOCKETS.lock().unwrap().get_mut().get_mut(idx).unwrap().bind(addr);

    return result_as_errno(res);
}

pub unsafe extern "C" fn dpoll_listen(socket_fd: c_int, backlog: c_int) -> c_int {
    let idx = buf::Index::from(socket_fd);
    assert!(idx.is_dpoll() );
assert!(idx.is_socket());

    let res = SOCKETS.lock().unwrap().get_mut().get_mut(idx).unwrap().listen(backlog);

    return result_as_errno(res);
}

pub unsafe extern "C" fn dpoll_accept(socket_fd: c_int, addr: *mut sockaddr, addr_len: *mut socklen_t) -> c_int {
    let addr = cast_sockaddr(addr, addr_len);
    let idx = buf::Index::from(socket_fd);
    assert!(idx.is_dpoll());
    assert!(idx.is_socket());

    let mut lock = SOCKETS.lock().unwrap();
    let sockets = lock.get_mut();
    let res = sockets.get_mut(idx).unwrap().accept(addr);
    let soc = match res {
        Ok(s) => s,
        Err(e) => return errno(e),
    };

    return sockets.allocate(soc).into();
}

pub unsafe extern "C" fn dpoll_close(fd: c_int) -> c_int {
    let idx: buf::Index = fd.into();

    let res = if idx.is_dpoll() {
        if idx.is_socket() {
            SOCKETS.lock().unwrap().get_mut().free(idx);
            0
        } else {
            todo!();
        }
    } else {
        unsafe {
            libc::close(fd)
        }
    };

    return res;
}

pub unsafe extern "C" fn dpoll_write(socket_fd: c_int, buf: *const c_void, len: size_t) -> ssize_t {
    let idx: buf::Index = socket_fd.into();
    assert!(idx.is_dpoll());
    assert!(idx.is_socket());

    let buf = unsafe { std::ptr::slice_from_raw_parts(buf as *const u8, len).as_ref() }.unwrap();
    let res = SOCKETS.lock().unwrap().get_mut().get_mut(idx).unwrap().write(buf);
    return match res {
        Ok(len) => len.try_into().unwrap(),
        Err(e) => errno(e) as isize,
    };
}

pub unsafe extern "C" fn dpoll_read(socket_fd: c_int, buf: *mut c_void, len: size_t) -> ssize_t {
    let idx: buf::Index = socket_fd.into();
    assert!(idx.is_dpoll());
    assert!(idx.is_socket());

    let buf = unsafe { std::ptr::slice_from_raw_parts_mut(buf as *mut MaybeUninit<u8>, len).as_mut() }.unwrap();
    let res = SOCKETS.lock().unwrap().get_mut().get_mut(idx).unwrap().read(buf);
    return match res {
        Ok(len) => len.try_into().unwrap(),
        Err(e) => errno(e) as isize,
    };
}

pub unsafe extern "C" fn dpoll_writev(socket_fd: c_int, vecs: *const iovec, len: c_int) -> ssize_t {
    todo!();
}

pub unsafe extern "C" fn dpoll_readv(socket_fd: c_int, vecs: *mut iovec, len: c_int) -> ssize_t {
    todo!();
}

pub unsafe extern "C" fn dpoll_init() -> c_int {
    return unsafe {
        result_as_errno(demi::meta_init())
    };
}

pub unsafe extern "C" fn dpoll_create(flags: c_int) -> c_int {
    todo!();
}

pub unsafe extern "C" fn dpoll_ctl(dpollfd: c_int, op: c_int, fd: c_int, event: *mut epoll_event) -> c_int {
    todo!();
}

pub unsafe extern "C" fn dpoll_pwait(dpollfd: c_int, events: *mut epoll_event, events_len: c_int, timeout: c_int, sigmask: *mut sigset_t) -> c_int {
    todo!();
}

