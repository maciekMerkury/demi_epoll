use super::{
    errno::{PosixError, PosixResult},
    helpers, raw,
};
use libc::{self, AF_INET, SOCK_STREAM, sockaddr_in};
use std::{
    mem::MaybeUninit,
    net::Ipv4Addr,
    os::raw::{c_int, c_uint},
    time::Duration,
};
use thiserror::Error;

pub type QToken = raw::demi_qtoken_t;
pub type Sgarray = raw::demi_sgarray;
pub type SocketQd = u32;

const ADDR_SIZE: u32 = std::mem::size_of::<raw::sockaddr_in>() as u32;

pub enum Opcode {
    INVALID = 0,
    PUSH = 1,
    POP = 2,
    ACCEPT = 3,
    CONNECT = 4,
    CLOSE = 5,
    FAILED = 6,
}

#[derive(Error, Debug)]
pub enum OpcError {
    #[error("{0} is too large to be an Opcode")]
    ValueOutOfRange(c_uint),
}

impl std::convert::Into<c_uint> for Opcode {
    fn into(self) -> c_uint {
        return self as c_uint;
    }
}

impl std::convert::TryFrom<c_uint> for Opcode {
    type Error = OpcError;

    fn try_from(value: c_uint) -> Result<Self, Self::Error> {
        return match value {
            0 => Ok(Opcode::INVALID),
            1 => Ok(Opcode::PUSH),
            2 => Ok(Opcode::POP),
            3 => Ok(Opcode::ACCEPT),
            4 => Ok(Opcode::CONNECT),
            5 => Ok(Opcode::CLOSE),
            6 => Ok(Opcode::FAILED),
            _ => Err(OpcError::ValueOutOfRange(value)),
        };
    }
}

pub struct AcceptResult {
    pub qd: Socket,
    pub addr: Ipv4Addr,
}

impl std::convert::From<raw::demi_accept_result> for AcceptResult {
    fn from(value: raw::demi_accept_result) -> Self {
        return Self {
            qd: value.qd.into(),
            addr: helpers::sockaddr_in_to_ipv4(value.addr).0,
        };
    }
}

pub enum QResultValue {
    Sga(Sgarray),
    Ares(AcceptResult),
}

pub struct QResult {
    pub opcode: Opcode,
    pub qd: Socket,
    pub qt: QToken,
    pub ret: i64,
    pub value: Option<QResultValue>,
}

impl std::convert::From<raw::demi_qresult> for QResult {
    fn from(value: raw::demi_qresult) -> Self {
        let opcode = value.qr_opcode.try_into().unwrap();
        let val = match opcode {
            Opcode::PUSH | Opcode::POP => Some(QResultValue::Sga(unsafe { value.qr_value.sga })),
            Opcode::ACCEPT => Some(QResultValue::Ares(unsafe { value.qr_value.ares }.into())),
            _ => None,
        };
        return Self {
            opcode,
            qd: value.qr_qd.into(),
            qt: value.qr_qt,
            ret: value.qr_ret,
            value: val,
        };
    }
}

#[inline]
pub fn meta_init() -> PosixResult<()> {
    let args = raw::demi_args {
        argc: 0,
        argv: std::ptr::null(),
        callback: None,
        logCallback: None,
    };

    return PosixError::from_errno(unsafe { raw::demi_init(&args) });
}

#[repr(transparent)]
pub struct Socket {
    pub qd: SocketQd,
}

impl std::convert::From<c_int> for Socket {
    fn from(value: c_int) -> Self {
        return Self { qd: value as u32 };
    }
}

impl Socket {
    #[inline]
    pub fn new() -> PosixResult<Self> {
        let mut qd: c_int = 0;
        PosixError::from_errno(unsafe { raw::demi_socket(&mut qd, AF_INET, SOCK_STREAM, 0) })?;
        return Ok(qd.into());
    }

    #[inline]
    pub fn listen(&mut self, backlog: i32) -> PosixResult<()> {
        return PosixError::from_errno(unsafe { raw::demi_listen(self.qd as c_int, backlog) });
    }

    #[inline]
    pub fn bind(&mut self, addr: &Ipv4Addr, port: u16) -> PosixResult<()> {
        let addr = helpers::ipv4_to_sockaddr_in(*addr, port);
        let addr_ptr = &addr as *const raw::sockaddr_in as *const raw::sockaddr;
        return PosixError::from_errno(unsafe {
            raw::demi_bind(self.qd as c_int, addr_ptr, ADDR_SIZE)
        });
    }

    #[inline]
    pub fn accept(&mut self) -> PosixResult<QToken> {
        let mut tok: QToken = 0;

        PosixError::from_errno(unsafe { raw::demi_accept(&mut tok, self.qd as c_int) })?;

        return Ok(tok);
    }

    #[inline]
    pub fn connect(&mut self, addr: &Ipv4Addr, port: u16) -> PosixResult<QToken> {
        let addr = helpers::ipv4_to_sockaddr_in(*addr, port);
        let addr_ptr = &addr as *const raw::sockaddr_in as *const raw::sockaddr;
        let mut tok: QToken = 0;
        PosixError::from_errno(unsafe {
            raw::demi_connect(&mut tok, self.qd as c_int, addr_ptr, ADDR_SIZE)
        })?;

        return Ok(tok);
    }

    #[inline]
    pub fn close(&mut self) -> PosixResult<()> {
        return PosixError::from_errno(unsafe { raw::demi_close(self.qd as c_int) });
    }

    #[inline]
    pub fn push(&mut self, sga: &Sgarray) -> PosixResult<QToken> {
        let mut tok: QToken = 0;
        PosixError::from_errno(unsafe { raw::demi_push(&mut tok, self.qd as c_int, sga) })?;

        return Ok(tok);
    }

    #[inline]
    pub fn pop(&mut self) -> PosixResult<QToken> {
        let mut tok: QToken = 0;
        PosixError::from_errno(unsafe { raw::demi_pop(&mut tok, self.qd as c_int) })?;

        return Ok(tok);
    }
}

pub fn wait(tok: QToken, timeout: Option<Duration>) -> PosixResult<QResult> {
    let mut res: MaybeUninit<raw::demi_qresult> = MaybeUninit::uninit();
    let ts: raw::timespec;
    let ts_ptr = if let Some(d) = timeout {
        ts = helpers::duration_to_timespec(d);
        &ts
    } else {
        std::ptr::null()
    };

    PosixError::from_errno(unsafe { raw::demi_wait(res.as_mut_ptr(), tok, ts_ptr) })?;
    return Ok(unsafe { res.assume_init() }.into());
}

pub fn wait_any(toks: &[QToken], timeout: Option<Duration>) -> PosixResult<(usize, QResult)> {
    let mut res: MaybeUninit<raw::demi_qresult> = MaybeUninit::uninit();
    let ts: raw::timespec;
    let ts_ptr = if let Some(d) = timeout {
        ts = helpers::duration_to_timespec(d);
        &ts
    } else {
        std::ptr::null()
    };
    let mut off = MaybeUninit::uninit();

    PosixError::from_errno(unsafe {
        raw::demi_wait_any(
            res.as_mut_ptr(),
            off.as_mut_ptr(),
            toks.as_ptr(),
            toks.len().try_into().unwrap(),
            ts_ptr,
        )
    })?;

    return Ok((
        unsafe { off.assume_init() }.try_into().unwrap(),
        unsafe { res.assume_init() }.into(),
    ));
}
