use std::os::raw::{c_int, c_void};
use libc::{sockaddr, socklen_t, ssize_t, size_t, iovec, epoll_event, sigset_t};

pub unsafe extern "C" fn dpoll_socket(domain: c_int, r#type: c_int, proto: c_int) -> c_int {
    todo!();
}

pub unsafe extern "C" fn dpoll_bind(socket_qd: c_int, addr: *const sockaddr, addr_len: socklen_t) -> c_int {
    todo!();
}

pub unsafe extern "C" fn dpoll_listen(socket_qd: c_int, backlog: c_int) -> c_int {
    todo!();
}

pub unsafe extern "C" fn dpoll_accept(socket_qd: c_int, addr: *mut sockaddr, addr_len: *mut socklen_t) -> c_int {
    todo!();
}

pub unsafe extern "C" fn dpoll_close(socket_qd: c_int) -> c_int {
    todo!();
}

pub unsafe extern "C" fn dpoll_write(socket_qd: c_int, buf: *const c_void, len: size_t) -> ssize_t {
    todo!();
}

pub unsafe extern "C" fn dpoll_read(socket_qd: c_int, buf: *mut c_void, len: size_t) -> ssize_t {
    todo!();
}

pub unsafe extern "C" fn dpoll_writev(socket_qd: c_int, vecs: *const iovec, len: c_int) -> ssize_t {
    todo!();
}

pub unsafe extern "C" fn dpoll_readv(socket_qd: c_int, vecs: *mut iovec, len: c_int) -> ssize_t {
    todo!();
}

pub unsafe extern "C" fn dpoll_init() -> c_int {
    todo!();
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

