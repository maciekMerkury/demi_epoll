use super::{
    errno::{PosixError, PosixResult},
    helpers::{self, WrapperConversion},
    raw::{self, demi_sgarray},
};
use libc::{self, AF_INET, SOCK_STREAM, iovec, sockaddr_in};
use log::trace;
use std::{
    mem::MaybeUninit,
    os::raw::{c_int, c_uint},
    time::Duration,
};
use thiserror::Error;

pub type QToken = raw::demi_qtoken_t;
pub type DemiQd = u32;

#[derive(Debug)]
pub struct SgArray {
    sga: raw::demi_sgarray,
}

impl std::convert::From<demi_sgarray> for SgArray {
    fn from(sga: demi_sgarray) -> Self {
        return Self { sga };
    }
}

impl SgArray {
    pub fn new(size: usize) -> Self {
        trace!("allocating {size} bytes");
        let s = Self {
            sga: unsafe { raw::demi_sgaalloc(size) },
        };

        assert!(s.sga.sga_numsegs > 0);

        return s;
    }

    pub fn len(&self) -> usize {
        return self.segments()
            .iter()
            .map(|s| s.data_len_bytes as usize)
            .sum();
    }

    pub fn from_slice(src: &[u8]) -> Self {
        let mut sga = Self::new(src.len());
        sga.fill(src);
        return sga;
    }

    pub fn from_slices(src: &[libc::iovec]) -> Self {
        let total_len = src.iter().map(|s| s.iov_len).sum();
        let mut sga = Self::new(total_len);
        sga.fill_from_slices(src);
        return sga;
    }

    fn segments(&self) -> &[raw::demi_sgaseg] {
        return &self.sga.segments[0..self.sga.sga_numsegs as usize];
    }

    /// will panic if `src.len() < self.len()`
    pub fn fill(&mut self, src: &[u8]) {
        assert!(src.len() >= self.len());

        let mut offset = 0;

        for seg in self.segments() {
            let len = seg.data_len_bytes as usize;
            let ptr = seg.data_buf_ptr as *mut u8;

            unsafe {
                std::ptr::copy_nonoverlapping(src.as_ptr().add(offset), ptr, len);
            }

            offset += len;
        }
    }

    /// will panic if `src.iter().map(|s| s.len()).sum() < self.len()`
    pub fn fill_from_slices(&mut self, mut src: &[libc::iovec]) {
        assert!(src.iter().map(|s| s.iov_len).sum::<usize>() >= self.len());

        let mut src_off = 0;
        for seg in self.segments() {
            let mut seg_off = 0;
            let len = seg.data_len_bytes as usize;
            let ptr = seg.data_buf_ptr as *mut u8;

            while seg_off < len {
                let bytes_left = len
                    .saturating_sub(seg_off)
                    .min(src[0].iov_len.saturating_sub(src_off));

                unsafe {
                    std::ptr::copy_nonoverlapping(
                        (src[0].iov_base as *const u8).add(src_off),
                        ptr.add(seg_off),
                        bytes_left,
                    );
                }

                seg_off += bytes_left;
                src_off += bytes_left;
                if src_off >= src[0].iov_len {
                    src = &src[1..];
                    src_off = 0;
                }
            }
        }
    }

    pub fn into_iter(self) -> SgArrayByteIter {
        return SgArrayByteIter::new(self);
    }
}

// impl Drop for SgArray {
//     fn drop(&mut self) {
//         assert!(unsafe { raw::demi_sgafree(&mut self.sga) } == 0);
//     }
// }

#[derive(Debug)]
pub struct SgArrayByteIter {
    sga: SgArray,
    /// offset into sga.segs
    seg_off: usize,
    /// offset into the segment
    byte_off: usize,
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
        let segs = self.sga.segments();
        return self.seg_off > segs.len() - 1;
    }

    /// copies K bytes into dst
    /// if the returned number of bytes is less than `dst.len()`, then `self.is_empty()` will be true
    pub fn copy_bytes(&mut self, mut dst: &mut [MaybeUninit<u8>]) -> Option<usize> {
        if self.is_empty() {
            return None;
        }

        let segs = self.sga.segments();

        let max_copied = dst.len();
        let mut total_copied = 0;
        while total_copied < max_copied && !self.is_empty() {
            let seg = &segs[self.seg_off];

            let bytes_left = (seg.data_len_bytes as usize).saturating_sub(self.byte_off);

            // no more data to copy
            if bytes_left == 0 {
                self.byte_off = 0;
                self.seg_off += 1;
                continue;
            }

            let copy_len = bytes_left.min(dst.len());

            unsafe {
                let src = seg.data_buf_ptr.add(self.byte_off) as *const u8;
                let dst = dst.as_mut_ptr() as *mut u8;

                std::ptr::copy_nonoverlapping(src, dst, copy_len);
            }

            self.byte_off += copy_len;
            total_copied += copy_len;
            dst = &mut dst[copy_len..];

            if self.byte_off >= seg.data_len_bytes as usize {
                self.seg_off += 1;
                self.byte_off = 0;
            }
        }

        return Some(total_copied);
    }

    pub fn copy_into_iovecs(&mut self, iovecs: &mut [iovec]) -> Option<usize> {
        if self.is_empty() {
            return None;
        }

        let mut total_copied = 0;

        for vec in iovecs.into_iter() {
            if self.is_empty() {
                break;
            }

            let vec = unsafe {
                std::ptr::slice_from_raw_parts_mut(
                    vec.iov_base as *mut MaybeUninit<u8>,
                    vec.iov_len,
                )
                .as_mut()
                .unwrap()
            };

            total_copied += self.copy_bytes(vec).unwrap();
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
    Push,
    Pop(SgArray),
    Accept(AcceptResult),
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct QResult {
    pub qd: DemiQd,
    pub qt: QToken,
    pub value: Option<QResultValue>,
}

impl std::convert::TryFrom<raw::demi_qresult> for QResult {
    type Error = PosixError;

    fn try_from(value: raw::demi_qresult) -> Result<Self, Self::Error> {
        let opcode = value.qr_opcode.try_into().unwrap();
        let val = match opcode {
            Opcode::PUSH => Ok(Some(QResultValue::Push)),
            Opcode::POP => Ok(Some(QResultValue::Pop(
                unsafe { value.qr_value.sga }.into(),
            ))),
            Opcode::ACCEPT => Ok(Some(QResultValue::Accept(
                unsafe { value.qr_value.ares }.into(),
            ))),
            Opcode::INVALID => panic!("invalid request to demikernel"),
            Opcode::CONNECT => Ok(None),
            Opcode::CLOSE => Ok(None),
            Opcode::FAILED => Err(
                PosixError::from_error_code(value.qr_ret.try_into().unwrap())
                    .err()
                    .unwrap(),
            ),
        }?;

        return Ok(Self {
            qd: value.qr_qd as u32,
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

    return PosixError::from_error_code(unsafe { raw::demi_init(&args) });
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
        PosixError::from_error_code(unsafe { raw::demi_socket(&mut qd, AF_INET, SOCK_STREAM, 0) })?;
        return Ok(qd.into());
    }

    #[inline]
    pub fn listen(&mut self, backlog: i32) -> PosixResult<()> {
        return PosixError::from_error_code(unsafe { raw::demi_listen(self.qd as c_int, backlog) });
    }

    #[inline]
    pub fn bind(&mut self, addr: *const libc::sockaddr_in) -> PosixResult<()> {
        let addr_ptr = addr as *const raw::sockaddr;
        return PosixError::from_error_code(unsafe {
            raw::demi_bind(self.qd as c_int, addr_ptr, ADDR_SIZE)
        });
    }

    #[inline]
    pub fn accept(&mut self) -> PosixResult<QToken> {
        let mut tok: QToken = 0;

        PosixError::from_error_code(unsafe { raw::demi_accept(&mut tok, self.qd as c_int) })?;

        return Ok(tok);
    }

    #[allow(dead_code)]
    #[inline]
    pub fn connect(&mut self, addr: *const libc::sockaddr_in) -> PosixResult<QToken> {
        let addr_ptr = addr as *const raw::sockaddr;
        let mut tok: QToken = 0;
        PosixError::from_error_code(unsafe {
            raw::demi_connect(&mut tok, self.qd as c_int, addr_ptr, ADDR_SIZE)
        })?;

        return Ok(tok);
    }

    #[inline]
    pub fn close(&mut self) -> PosixResult<()> {
        return PosixError::from_error_code(unsafe { raw::demi_close(self.qd as c_int) });
    }

    #[inline]
    pub fn push(&mut self, sga: &SgArray) -> PosixResult<QToken> {
        let mut tok: QToken = 0;
        PosixError::from_error_code(unsafe {
            raw::demi_push(&mut tok, self.qd as c_int, &sga.sga)
        })?;

        return Ok(tok);
    }

    #[inline]
    pub fn pop(&mut self) -> PosixResult<QToken> {
        let mut tok: QToken = 0;
        PosixError::from_error_code(unsafe { raw::demi_pop(&mut tok, self.qd as c_int) })?;

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

    PosixError::from_error_code(unsafe { raw::demi_wait(res.as_mut_ptr(), tok, ts_ptr) })?;
    return unsafe { res.assume_init() }.try_into();
}

pub fn wait_any(
    toks: &[QToken],
    timeout: Option<Duration>,
) -> PosixResult<(usize, PosixResult<QResult>)> {
    let mut res: MaybeUninit<raw::demi_qresult> = MaybeUninit::uninit();
    let ts: raw::timespec;
    let ts_ptr = if let Some(d) = timeout {
        ts = helpers::duration_to_timespec(d);
        &ts
    } else {
        std::ptr::null()
    };
    let mut off = MaybeUninit::uninit();
    trace!("wait_any on {} toks, timeout: {:?}", toks.len(), timeout);

    PosixError::from_error_code(unsafe {
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
