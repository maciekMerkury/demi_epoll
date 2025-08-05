mod utils;
use env_logger::{Builder, Env};
use lazy_static::lazy_static;
use log::trace;
use utils::{cast_sockaddr, errno, result_as_errno};

use crate::{
    buffer::{self as buf, Buffer},
    dpoll::{self, Dpoll, DpollErrors},
    socket::Socket,
    wrappers::{
        self, demi,
        errno::{PosixError, PosixResult}, sigmask::Sigset,
    },
};
use core::slice;
use libc::{
    AF_INET, SOCK_STREAM, epoll_event, iovec, sigset_t, size_t, sockaddr, sockaddr_in, socklen_t,
    ssize_t,
};
use std::{
    cell::RefCell, env, io::Write, mem::{self, MaybeUninit}, ops::Deref, os::raw::{c_int, c_void}, sync::{Arc, Mutex, RwLock}, time::Duration
};

#[inline]
const fn new_buffer<const S: bool, T>() -> RwLock<Buffer<S, T>> {
    return RwLock::new(Buffer::new());
}

static DPOLLS: RwLock<Buffer<false, Arc<Mutex<Dpoll>>>> = new_buffer();
static SOCKETS: RwLock<Buffer<true, Arc<Mutex<Socket>>>> = new_buffer();

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dpoll_socket(domain: c_int, r#type: c_int, proto: c_int) -> c_int {
    trace!("creating new socket");
    assert!(domain == AF_INET);
    assert!(r#type == SOCK_STREAM);
    let soc = match Socket::socket() {
        Ok(s) => s,
        Err(e) => return errno(e),
    };
    let idx = SOCKETS.write().unwrap().allocate(Arc::new(Mutex::new(soc)));
    return idx.into();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dpoll_bind(
    socket_fd: c_int,
    addr: *const sockaddr,
    addr_len: socklen_t,
) -> c_int {
    trace!("bind");
    assert!(addr_len as usize == mem::size_of::<libc::sockaddr_in>());
    let addr = unsafe { (addr as *const sockaddr_in).as_ref() }.unwrap();

    let idx = buf::Index::from(socket_fd);

    let res = SOCKETS
        .read()
        .unwrap()
        .get(idx)
        .unwrap()
        .try_lock()
        .unwrap()
        .bind(addr);

    return result_as_errno(res);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dpoll_listen(socket_fd: c_int, backlog: c_int) -> c_int {
    trace!("");
    let idx = buf::Index::from(socket_fd);

    let res = SOCKETS
        .read()
        .unwrap()
        .get(idx)
        .unwrap()
        .try_lock()
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
    trace!("");
    let addr = cast_sockaddr(addr, addr_len);
    let idx = buf::Index::from(socket_fd);

    let mut lock = SOCKETS.write().unwrap();
    let res = lock.get_mut(idx).unwrap().try_lock().unwrap().accept(addr);
    let soc = match res {
        Ok(s) => s,
        Err(e) => return errno(e),
    };

    return lock.allocate(Arc::new(Mutex::new(soc))).into();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dpoll_close(fd: c_int) -> c_int {
    trace!("");
    let idx: buf::Index = fd.into();

    let res = if idx.is_dpoll() {
        if idx.is_socket() {
            let mut sockets = SOCKETS.write().unwrap();
            sockets.get(idx).unwrap().lock().unwrap().close();
            sockets.free(idx);
        } else {
            DPOLLS.write().unwrap().free(idx);
        }
        0
    } else {
        unsafe { libc::close(fd) }
    };

    return res;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dpoll_write(socket_fd: c_int, buf: *const c_void, len: size_t) -> ssize_t {
    let idx: buf::Index = socket_fd.into();

    if !idx.is_dpoll() {
        return unsafe { libc::write(socket_fd, buf, len) };
    }

    let buf = unsafe { std::ptr::slice_from_raw_parts(buf as *const u8, len).as_ref() }.unwrap();
    let res = SOCKETS
        .read()
        .unwrap()
        .get(idx)
        .unwrap()
        .try_lock()
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

    if !idx.is_dpoll() {
        return unsafe { libc::read(socket_fd, buf, len) };
    }

    let buf =
        unsafe { std::ptr::slice_from_raw_parts_mut(buf as *mut MaybeUninit<u8>, len).as_mut() }
            .unwrap();

    let res = SOCKETS
        .read()
        .unwrap()
        .get(idx)
        .unwrap()
        .try_lock()
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

    if !idx.is_dpoll() {
        return unsafe { libc::writev(socket_fd, vecs, iovec_count) };
    }

    let vecs =
        unsafe { std::ptr::slice_from_raw_parts(vecs, iovec_count.try_into().unwrap()).as_ref() }
            .unwrap();

    let res = SOCKETS
        .read()
        .unwrap()
        .get(idx)
        .unwrap()
        .try_lock()
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

    if !idx.is_dpoll() {
        return unsafe { libc::readv(socket_fd, vecs, iovec_count) };
    }

    let vecs = unsafe {
        std::ptr::slice_from_raw_parts_mut(vecs, iovec_count.try_into().unwrap()).as_mut()
    }
    .unwrap();

    let res = SOCKETS
        .read()
        .unwrap()
        .get(idx)
        .unwrap()
        .try_lock()
        .unwrap()
        .readv(vecs);

    return match res {
        Ok(len) => len.try_into().unwrap(),
        Err(e) => errno(e) as isize,
    };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dpoll_init() -> c_int {
    if unsafe { result_as_errno(demi::meta_init()) }.is_negative() {
        return -1;
    }

    let mut builder = Builder::new();
    if let Ok(log) = env::var("DPOLL_LOG") {
        builder.parse_filters(&log);
    } else {
        builder.parse_default_env();
    }

    builder.format(|buf, record| {
        let ts = buf.timestamp();
        writeln!(
            buf,
            "[{ts} {level} {file}:{line} {path}] {args}",
            level = record.level(),
            file = record.file().unwrap_or("unknown"),
            line = record.line().unwrap_or(0),
            path = record.target(), // Optional: module path / target
            args = record.args()
        )
    });

    builder.init();

    return 0;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dpoll_create(flags: c_int) -> c_int {
    let pol = match Dpoll::create(flags) {
        Ok(s) => s,
        Err(e) => return errno(e),
    };

    let idx = DPOLLS
        .write()
        .unwrap()
        .allocate(Arc::new(Mutex::new(pol)));
    trace!("{:?}", idx);
    return idx.into();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dpoll_ctl(
    dpollfd: c_int,
    op: c_int,
    fd: c_int,
    event: *mut epoll_event,
) -> c_int {
    trace!("op: {:?}, fd: {:?}", op, fd);
    let pol: buf::Index = dpollfd.into();
    let soc: buf::Index = fd.into();
    let sockets = SOCKETS.read().unwrap();
    let qd = sockets
        .get(soc)
        .map(|s| s.try_lock().unwrap().soc.qd);

    let op = dpoll::Operation::new(soc, qd, op, unsafe { event.as_ref() }).unwrap();
    let res = DPOLLS
        .read()
        .unwrap()
        .get(pol)
        .unwrap()
        .try_lock()
        .unwrap()
        .ctl(&sockets, op);
    trace!("dpoll {pol:?} returned {res:?}");
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
    let old_set = Sigset::mask(sigmask);
    let pol: buf::Index = dpollfd.into();

    let evs = unsafe {
        std::ptr::slice_from_raw_parts_mut(
            events as *mut MaybeUninit<epoll_event>,
            events_len.try_into().unwrap(),
        )
        .as_mut()
    }
    .unwrap();
    let timeout = if timeout.is_negative() {
        None
    } else {
        Some(Duration::from_millis(timeout as u64))
    };
    let sigmask = unsafe { sigmask.as_ref() };

    let pol = DPOLLS.read().unwrap().get(pol).unwrap().clone();
    trace!("pwait on {pol:?} for {timeout:?}");
    let res = pol
        .try_lock()
        .unwrap()
        .pwait(evs, timeout);

    trace!("pwait on {pol:?} returned {res:?}");
    return match res {
        Ok(count) => count.try_into().unwrap(),
        Err(PosixError::TIMEDOUT) => 0,
        Err(err) => errno(err),
    };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dpoll_setsockopt(
    socket: c_int,
    level: c_int,
    optname: c_int,
    optval: *const c_void,
    optlen: socklen_t,
) -> c_int {
    trace!("");
    let idx: buf::Index = socket.into();
    return if idx.is_dpoll() {
        0
    } else {
        unsafe { libc::setsockopt(socket, level, optname, optval, optlen) }
    };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dpoll_getsockname(
    socket: c_int,
    addr: *mut sockaddr,
    len: *mut socklen_t,
) -> c_int {
    trace!("");
    let idx: buf::Index = socket.into();
    let soc_addr = SOCKETS
        .read()
        .unwrap()
        .get(idx)
        .unwrap()
        .try_lock()
        .unwrap()
        .addr
        .unwrap();
    unsafe {
        (addr as *mut sockaddr_in).write(soc_addr);
        len.write(mem::size_of::<libc::sockaddr_in>() as u32);
    }

    return 0;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dpoll_sendmsg(
    socket: c_int,
    msg: *const libc::msghdr,
    flags: c_int,
) -> c_int {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dpoll_recvmsg(
    socket: c_int,
    msg: *mut libc::msghdr,
    flags: c_int,
) -> c_int {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dpoll_connect(
    socket_fd: c_int,
    addr: *const sockaddr,
    len: socklen_t,
) -> c_int {
    unimplemented!();
}
