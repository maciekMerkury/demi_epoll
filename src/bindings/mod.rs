mod utils;
use env_logger::{Builder, Env};
use lazy_static::lazy_static;
use log::trace;
use utils::{cast_sockaddr, errno, result_as_errno};

use crate::{
    buffer::{self as buf, Index},
    dpoll::{self, Dpoll},
    shared::{Shared, ThreadBuffer, new_thread_buffer},
    socket::Socket,
    wrappers::{
        demi,
        errno::{PosixError, PosixResult},
        sigmask::Sigset,
    },
};
use core::slice;
use libc::{
    AF_INET, SOCK_STREAM, epoll_event, iovec, sigset_t, size_t, sockaddr, sockaddr_in, socklen_t,
    ssize_t,
};
use std::{
    cell::RefCell,
    env,
    io::Write,
    mem::{self, MaybeUninit},
    os::raw::{c_int, c_void},
    rc::Rc,
    time::Duration,
};

thread_local! {
    static DPOLLS: ThreadBuffer<false, Dpoll> = const { new_thread_buffer() };
    static SOCKETS: ThreadBuffer<true, Socket> = const { new_thread_buffer() };
}

#[unsafe(no_mangle)]
pub extern "C" fn dpoll_socket(domain: c_int, r#type: c_int, proto: c_int) -> c_int {
    trace!("creating new socket");
    assert!(domain == AF_INET);
    assert!(r#type == SOCK_STREAM);
    let soc = match Socket::socket() {
        Ok(s) => s,
        Err(e) => return errno(e),
    };
    let idx = SOCKETS.with_borrow_mut(|socs| socs.allocate(Shared::new(soc)));
    trace!("new socket {idx:?} created");
    return idx.into();
}

#[unsafe(no_mangle)]
pub extern "C" fn dpoll_bind(
    socket_fd: c_int,
    addr: *const sockaddr,
    addr_len: socklen_t,
) -> c_int {
    assert!(addr_len as usize == mem::size_of::<libc::sockaddr_in>());
    let addr = unsafe { (addr as *const sockaddr_in).as_ref() }.unwrap();

    let idx = buf::Index::from(socket_fd);
    trace!("bind on {idx:?}");

    let res = SOCKETS.with_borrow(|socs| socs.get(idx).unwrap().borrow_mut().bind(addr));

    return result_as_errno(res);
}

#[unsafe(no_mangle)]
pub extern "C" fn dpoll_listen(socket_fd: c_int, backlog: c_int) -> c_int {
    let idx = buf::Index::from(socket_fd);
    trace!("listen on {idx:?}");

    let res = SOCKETS.with_borrow(|socs| socs.get(idx).unwrap().borrow_mut().listen(backlog));

    return result_as_errno(res);
}

#[unsafe(no_mangle)]
pub extern "C" fn dpoll_accept(
    socket_fd: c_int,
    addr: *mut sockaddr,
    addr_len: *mut socklen_t,
) -> c_int {
    let addr = cast_sockaddr(addr, addr_len);
    let idx = buf::Index::from(socket_fd);

    trace!("accept on {idx:?}");
    let new: PosixResult<Index> = SOCKETS.with_borrow_mut(|socs| {
        let res = socs.get_mut(idx).unwrap().borrow_mut().accept(addr);
        let soc = res?;

        return Ok(socs.allocate(Shared::new(soc)));
    });
    trace!("accepted {new:?}");

    return match new {
        Ok(idx) => idx.into(),
        Err(e) => errno(e),
    };
}

#[unsafe(no_mangle)]
pub extern "C" fn dpoll_close(fd: c_int) -> c_int {
    trace!("closing {fd}");
    let idx: buf::Index = fd.into();

    let res = if !idx.is_dpoll() {
        unsafe { libc::close(fd) }
    } else {
        if idx.is_socket() {
            SOCKETS.with_borrow_mut(|socs| socs.take(idx).borrow_mut().close());
        } else {
            DPOLLS.with_borrow_mut(|polls| polls.free(idx))
        }
        0
    };

    trace!("closed {fd}, ret: {res}");
    return res;
}

#[unsafe(no_mangle)]
pub extern "C" fn dpoll_write(socket_fd: c_int, buf: *const c_void, len: size_t) -> ssize_t {
    assert!(!buf.is_null());
    let idx: buf::Index = socket_fd.into();

    trace!("writing {len} bytes to {idx:?}");

    if !idx.is_dpoll() {
        return unsafe { libc::write(socket_fd, buf, len) };
    }

    if len == 0 {
        return 0;
    }

    let buf = unsafe { std::ptr::slice_from_raw_parts(buf as *const u8, len).as_ref() }.unwrap();
    let res = SOCKETS.with_borrow_mut(|socs| socs.get(idx).unwrap().borrow_mut().write(buf));

    trace!("write res: {res:?}");
    return match res {
        Ok(len) => len.try_into().unwrap(),
        Err(e) => errno(e) as isize,
    };
}

#[unsafe(no_mangle)]
pub extern "C" fn dpoll_read(socket_fd: c_int, buf: *mut c_void, len: size_t) -> ssize_t {
    assert!(!buf.is_null());
    let idx: buf::Index = socket_fd.into();

    trace!("reading {len} bytes to {idx:?}");

    if !idx.is_dpoll() {
        return unsafe { libc::read(socket_fd, buf, len) };
    }

    if len == 0 {
        return 0;
    }

    let buf =
        unsafe { std::ptr::slice_from_raw_parts_mut(buf as *mut MaybeUninit<u8>, len).as_mut() }
            .unwrap();

    let res = SOCKETS.with_borrow_mut(|socs| socs.get(idx).unwrap().borrow_mut().read(buf));

    trace!("read res: {res:?}");
    return match res {
        Ok(len) => len.try_into().unwrap(),
        Err(e) => errno(e) as isize,
    };
}

#[unsafe(no_mangle)]
pub extern "C" fn dpoll_writev(
    socket_fd: c_int,
    vecs: *const iovec,
    iovec_count: c_int,
) -> ssize_t {
    assert!(!vecs.is_null());
    let idx: buf::Index = socket_fd.into();

    trace!("writev of {iovec_count} to {idx:?}");

    if !idx.is_dpoll() {
        return unsafe { libc::writev(socket_fd, vecs, iovec_count) };
    }

    if iovec_count == 0 || unsafe { *vecs }.iov_len == 0 {
        return 0
    }

    let vecs =
        unsafe { std::ptr::slice_from_raw_parts(vecs, iovec_count.try_into().unwrap()).as_ref() }
            .unwrap();

    let res = SOCKETS.with_borrow_mut(|socs| socs.get(idx).unwrap().borrow_mut().writev(vecs));

    trace!("writev res: {res:?}");
    return match res {
        Ok(len) => len.try_into().unwrap(),
        Err(e) => errno(e) as isize,
    };
}

#[unsafe(no_mangle)]
pub extern "C" fn dpoll_readv(
    socket_fd: c_int,
    vecs: *mut iovec,
    iovec_count: c_int,
) -> ssize_t {
    assert!(!vecs.is_null());
    let idx: buf::Index = socket_fd.into();

    trace!("readv of {iovec_count} to {idx:?}");

    if !idx.is_dpoll() {
        return unsafe { libc::readv(socket_fd, vecs, iovec_count) };
    }

    if iovec_count == 0 || unsafe { *vecs }.iov_len == 0 {
        return 0
    }

    let vecs = unsafe {
        std::ptr::slice_from_raw_parts_mut(vecs, iovec_count.try_into().unwrap()).as_mut()
    }
    .unwrap();

    let res = SOCKETS.with_borrow_mut(|socs| socs.get(idx).unwrap().borrow_mut().readv(vecs));

    trace!("readv res: {res:?}");
    return match res {
        Ok(len) => len.try_into().unwrap(),
        Err(e) => errno(e) as isize,
    };
}

#[unsafe(no_mangle)]
pub extern "C" fn dpoll_init() -> c_int {
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
            path = record.target(),
            args = record.args()
        )
    });

    builder.init();

    return 0;
}

#[unsafe(no_mangle)]
pub extern "C" fn dpoll_create(flags: c_int) -> c_int {
    let pol = match Dpoll::create(flags) {
        Ok(s) => s,
        Err(e) => return errno(e),
    };

    let idx = DPOLLS.with_borrow_mut(|polls| polls.allocate(Shared::new(pol)));

    trace!("{:?}", idx);
    return idx.into();
}

#[unsafe(no_mangle)]
pub extern "C" fn dpoll_ctl(
    dpollfd: c_int,
    op: c_int,
    fd: c_int,
    event: *mut epoll_event,
) -> c_int {
    let pol: buf::Index = dpollfd.into();
    let soc: buf::Index = fd.into();
    trace!("ctl pol {pol:?} on soc {soc:?}");

    let op = SOCKETS.with_borrow(|socs| unsafe { dpoll::Operation::from_raw(socs, op, fd, event) });
    let res = DPOLLS.with_borrow_mut(|polls| polls.get(pol).unwrap().borrow_mut().ctl(op));
    return result_as_errno(res);
}

#[unsafe(no_mangle)]
pub extern "C" fn dpoll_pwait(
    dpollfd: c_int,
    events: *mut epoll_event,
    events_len: c_int,
    timeout: c_int,
    sigmask: *const sigset_t,
) -> c_int {
    let old_set = Sigset::mask(sigmask);
    let pol: buf::Index = dpollfd.into();

    assert!(!events.is_null());
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

    let tmp = pol;
    let pol = DPOLLS.with_borrow(|polls| polls.get(pol).unwrap().clone());
    trace!("pwait on {tmp:?} for {timeout:?}");
    let res = pol.borrow_mut().pwait(evs, timeout);

    trace!("pwait on {tmp:?} returned {res:?}");
    return match res {
        Ok(count) => count.try_into().unwrap(),
        Err(PosixError::TIMEDOUT) => 0,
        Err(err) => errno(err),
    };
}

#[unsafe(no_mangle)]
pub extern "C" fn dpoll_setsockopt(
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
pub extern "C" fn dpoll_getsockname(
    socket: c_int,
    addr: *mut sockaddr,
    len: *mut socklen_t,
) -> c_int {
    assert!(!len.is_null() && !addr.is_null());
    assert!(unsafe { *len } as usize >= mem::size_of::<sockaddr_in>());
    let addr = addr as *mut sockaddr_in;

    let idx: buf::Index = socket.into();
    let soc_addr = SOCKETS.with_borrow(|socs| socs.get(idx).unwrap().borrow().addr.unwrap());
    unsafe {
        addr.write(soc_addr);
        len.write(mem::size_of::<libc::sockaddr_in>() as u32);
    }

    return 0;
}

#[unsafe(no_mangle)]
pub extern "C" fn dpoll_sendmsg(
    socket: c_int,
    msg: *const libc::msghdr,
    flags: c_int,
) -> c_int {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub extern "C" fn dpoll_recvmsg(
    socket: c_int,
    msg: *mut libc::msghdr,
    flags: c_int,
) -> c_int {
    unimplemented!();
}

#[unsafe(no_mangle)]
pub extern "C" fn dpoll_connect(
    socket_fd: c_int,
    addr: *const sockaddr,
    len: socklen_t,
) -> c_int {
    unimplemented!();
}
