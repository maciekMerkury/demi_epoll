
use std::mem::MaybeUninit;

use crate::operation::Operation;

use crate::wrappers::errno::PosixError;
use crate::wrappers::{demi, errno::PosixResult};

enum SocketData {
    Passive {
        accept: Operation<(), demi::AcceptResult>
    },

    Active {
        write: Operation<demi::SgArray, ()>,
        read: Operation<(), demi::SgArrayByteIter>,
    },
}

impl SocketData {
    pub const fn new_passive() -> Self {
        return Self::Passive { accept: Operation::default() };
    }

    pub const fn new_active() -> Self {
        return Self::Active { write: Operation::default(), read: Operation::default() };
    }
}

pub struct Socket {
    soc: demi::SocketQd,
    /// to be used with getsockname
    addr: Option<libc::sockaddr_in>,

    data: SocketData,
}

impl Drop for Socket {
    fn drop(&mut self) {
        match &mut self.data {
            SocketData::Passive { accept } => accept.block(),
            SocketData::Active { write, read } => {
                write.block();
                read.block();
            }
        }
    }
}

impl Socket {
    pub fn socket() -> PosixResult<Self> {
        return demi::SocketQd::new().map(Self::new);
    }

    pub fn new(soc: demi::SocketQd) -> Self {
        return Self {
            soc,
            addr: None,
            data: SocketData::Passive { accept: Operation::None },
        };
    }

    #[inline]
    pub fn bind(&mut self, addr: &libc::sockaddr_in) -> PosixResult<()> {
        self.soc.bind(addr)?;
        self.data = SocketData::new_passive();
        self.addr = Some(*addr);
        
        return Ok(());
    }

    #[inline]
    pub fn listen(&mut self, backlog: i32) -> PosixResult<()> {
        return self.soc.listen(backlog);
    }

    pub fn accept(&mut self, addr: Option<&mut MaybeUninit<libc::sockaddr_in>>) -> PosixResult<Self> {
        let data = match &mut self.data {
            SocketData::Passive { accept } => accept,
            _ => return Err(PosixError::INVAL),
        };

        let soc: Socket = data.get_or_schedule(|| (self.soc.accept().unwrap(), ())).unwrap_or(Err(PosixError::WOULDBLOCK)).map(From::from)?;
        if let Some(addr) = addr {
            addr.write(soc.addr.unwrap());
        }
        return Ok(soc);
    }

    pub fn write(&mut self, src: &[u8]) -> PosixResult<usize> {
        let write = match &mut self.data {
            SocketData::Active { write, read } => write,
            _ => return Err(PosixError::INVAL),
        };

        if write.poll() {
            write.get().unwrap();
        } else {
            return Err(PosixError::WOULDBLOCK);
        }

        let sga = demi::SgArray::from_slice(src);
        write.schedule(self.soc.push(&sga).unwrap(), sga);

        return Ok(src.len());
    }

    pub fn read(&mut self, dst: &mut [MaybeUninit<u8>]) -> PosixResult<usize> {
        let read = match &mut self.data {
            SocketData::Active { write, read } => read,
            _ => return Err(PosixError::INVAL),
        };

        if !read.poll() {
            read.schedule(self.soc.pop().unwrap(), ());
            return Err(PosixError::WOULDBLOCK);
        }
        let iter = read.get_mut().unwrap();
        let len = iter.copy_bytes(dst).unwrap();
        if iter.is_empty() {
            let _ = read.get();
            read.schedule(self.soc.pop().unwrap(), ());
        }

        return Ok(len);
    }
}

impl std::convert::From<demi::AcceptResult> for Socket {
    fn from(value: demi::AcceptResult) -> Self {
        return Self {
            soc: value.qd,
            addr: Some(value.addr),
            data: SocketData::new_active(),
        };
    }
}
