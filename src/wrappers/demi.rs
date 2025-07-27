use super::{
    errno::{PosixError, PosixResult},
    helpers, raw::{self, demi_sgarray, demi_sgaseg},
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
type DemiQd = u32;

#[derive(Debug)]
pub struct SgArray {
    sga: raw::demi_sgarray,
}

impl std::convert::From<demi_sgarray> for SgArray {
    fn from(sga: demi_sgarray) -> Self {
        return Self {
            sga,
        };
    }
}

impl SgArray {
    pub fn new(size: usize) -> Self {
        let s = Self {
            sga: unsafe { raw::demi_sgaalloc(size) }
        };

        assert!(s.sga.sga_numsegs > 0);

        return s;
    }

    pub fn len(&self) -> usize {
        return unsafe { self.segments() }.iter().map(|s| s.sgaseg_len as usize).sum();
    }

    unsafe fn segments(&self) -> &[raw::demi_sgaseg] {
        return unsafe {
            std::slice::from_raw_parts(self.sga.sga_segs.as_ptr(), self.sga.sga_numsegs as usize)
        };
    }

    /// will panic if `src.len() < self.len()`
    pub fn fill(&mut self, src: &[u8]) {
        assert!(src.len() >= self.len());

        let mut offset = 0;

        for seg in unsafe { self.segments() } {
            let len = seg.sgaseg_len as usize;
            let ptr = seg.sgaseg_buf as *mut u8;

            unsafe {
                std::ptr::copy_nonoverlapping(src.as_ptr().add(offset), ptr, len);
            }

            offset += len;
        }
    }

    pub fn into_iter(self) -> SgArrayByteIter {
        return SgArrayByteIter::new(self);
    }
}

impl Drop for SgArray {
    fn drop(&mut self) {
        assert!(unsafe { raw::demi_sgafree(&mut self.sga) } == 0);
    }
}

#[derive(Debug)]
pub struct SgArrayByteIter {
    sga: SgArray,
    /// offset into sga.segs
    seg_off: usize,
    /// offset into the segment
    byte_off: usize
}

impl SgArrayByteIter {
    fn new(sga: SgArray) -> Self {
        return Self {
            sga,
            seg_off: 0,
            byte_off: 0,
        };
    }

    pub fn is_empty(&self) -> bool {
        let segs = unsafe { self.sga.segments() };
        return self.seg_off == segs.len() - 1 && self.byte_off > segs[self.seg_off].sgaseg_len as usize;
    }

    /// copies K bytes into dst
    /// if the returned number of bytes is less than `dst.len()`, then `self.is_empty()` will be true
    pub fn copy_bytes(&mut self, mut dst: &mut [u8]) -> Option<usize> {
        if self.is_empty() {
            return None;
        }

        let segs = unsafe { self.sga.segments() };

        let max_copied = dst.len();
        let mut total_copied = 0;
        while total_copied < max_copied && self.seg_off < segs.len() {
            let seg = &segs[self.seg_off];

            let bytes_left = (seg.sgaseg_len as usize).saturating_sub(self.byte_off);

            // no more data to copy
            if bytes_left == 0 {
                self.byte_off = 0;
                self.seg_off += 1;
                continue;
            }

            let copy_len = bytes_left.min(dst.len());

            unsafe {
                let src = seg.sgaseg_buf.add(self.byte_off) as *const u8;
                let dst = dst.as_mut_ptr();

                std::ptr::copy_nonoverlapping(src, dst, copy_len);
            }

            self.byte_off += copy_len;
            total_copied += copy_len;
            dst = &mut dst[copy_len..];

            if self.byte_off >= seg.sgaseg_len as usize {
                self.seg_off += 1;
                self.byte_off = 0;
            }
        }

        return Some(total_copied);
    }

}

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

#[derive(Debug)]
pub struct AcceptResult {
    pub qd: SocketQd,
    pub addr: sockaddr_in,
}

impl std::convert::From<raw::demi_accept_result> for AcceptResult {
    fn from(value: raw::demi_accept_result) -> Self {
        return Self {
            qd: value.qd.into(),
            addr: value.addr.cast(),
        };
    }
}

#[derive(Debug)]
pub enum QResultValue {
    Push(SgArray),
    Pop(SgArray),
    Accept(AcceptResult),
}

#[derive(Debug)]
pub struct QResult {
    pub qd: SocketQd,
    pub qt: QToken,
    pub value: Option<QResultValue>,
}

impl std::convert::TryFrom<raw::demi_qresult> for QResult {
    type Error = PosixError;

    fn try_from(value: raw::demi_qresult) -> Result<Self, Self::Error> {
        let opcode = value.qr_opcode.try_into().unwrap();
        let val = match opcode {
            Opcode::PUSH => Ok(Some(QResultValue::Push(unsafe{ value.qr_value.sga }.into()))),
            Opcode::POP => Ok(Some(QResultValue::Pop(unsafe{ value.qr_value.sga }.into()))),
            Opcode::ACCEPT => Ok(Some(QResultValue::Accept(unsafe{ value.qr_value.ares }.into()))),
            Opcode::INVALID => panic!("invalid request to demikernel"),
            Opcode::CONNECT => Ok(None),
            Opcode::CLOSE => Ok(None),
            Opcode::FAILED => Err(PosixError::from_errno(value.qr_ret.try_into().unwrap()).err().unwrap()),
        }?;

        return Ok(Self {
            qd: value.qr_qd.into(),
            qt: value.qr_qt,
            value: val,
        });
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
#[derive(Debug)]
pub struct SocketQd {
    pub qd: DemiQd,
}

impl std::convert::From<c_int> for SocketQd {
    fn from(value: c_int) -> Self {
        return Self { qd: value as u32 };
    }
}

impl SocketQd {
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
    pub fn bind(&mut self, addr: *const libc::sockaddr_in) -> PosixResult<()> {
        let addr_ptr = addr as *const raw::sockaddr;
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
    pub fn connect(&mut self, addr: *const libc::sockaddr_in) -> PosixResult<QToken> {
        let addr_ptr = addr as *const raw::sockaddr;
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
    pub fn push(&mut self, sga: &SgArray) -> PosixResult<QToken> {
        let mut tok: QToken = 0;
        PosixError::from_errno(unsafe { raw::demi_push(&mut tok, self.qd as c_int, &sga.sga) })?;

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
    return unsafe { res.assume_init() }.try_into();
}

pub fn wait_any(toks: &[QToken], timeout: Option<Duration>) -> PosixResult<(usize, PosixResult<QResult>)> {
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
        unsafe { res.assume_init() }.try_into(),
    ));
}
