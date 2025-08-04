use std::mem::MaybeUninit;
use std::usize;

use bitflags::Flags;

use crate::dpoll::Event;
use crate::operation::Operation;

use crate::wrappers::demi::QResultValue;
use crate::wrappers::errno::PosixError;
use crate::wrappers::{demi, errno::PosixResult};

enum SocketData {
    Passive {
        accept: Operation<demi::AcceptResult>,
    },

    Active {
        write: Operation<()>,
        read: Operation<demi::SgArrayByteIter>,
    },
}

impl SocketData {
    pub const fn new_passive() -> Self {
        return Self::Passive {
            accept: Operation::default(),
        };
    }

    pub const fn new_active() -> Self {
        return Self::Active {
            write: Operation::default(),
            read: Operation::default(),
        };
    }
}

pub struct Socket {
    pub soc: demi::SocketQd,
    /// to be used with getsockname
    pub addr: Option<libc::sockaddr_in>,

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
            data: SocketData::Passive {
                accept: Operation::None,
            },
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

    pub fn accept(
        &mut self,
        addr: Option<&mut MaybeUninit<libc::sockaddr_in>>,
    ) -> PosixResult<Self> {
        let data = match &mut self.data {
            SocketData::Passive { accept } => accept,
            _ => return Err(PosixError::INVAL),
        };

        let soc: Socket = data
            .get_or_schedule(|| (&mut self.soc, ()))
            .unwrap_or(Err(PosixError::WOULDBLOCK))
            .map(From::from)?;
        if let Some(addr) = addr {
            addr.write(soc.addr.unwrap());
        }
        return Ok(soc);
    }

    pub fn write(&mut self, src: &[u8]) -> PosixResult<usize> {
        return self.write_impl(|| demi::SgArray::from_slice(src));
    }

    pub fn writev(&mut self, src: &[libc::iovec]) -> PosixResult<usize> {
        return self.write_impl(|| demi::SgArray::from_slices(src));
    }

    pub fn read(&mut self, dst: &mut [MaybeUninit<u8>]) -> PosixResult<usize> {
        return self.read_impl(|it| it.copy_bytes(dst).unwrap());
    }

    pub fn readv(&mut self, dst: &mut [libc::iovec]) -> PosixResult<usize> {
        return self.read_impl(|it| it.copy_into_iovecs(dst).unwrap());
    }

    pub fn available_events(&self, evs: Event) -> Event {
        let other = match &self.data {
            SocketData::Passive { accept } => {
                if accept.is_finished() {
                    Event::IN
                } else {
                    Event::empty()
                }
            }
            SocketData::Active { write, read } => {
                let write = if !write.is_running() {
                    Event::OUT
                } else {
                    Event::empty()
                };
                let read = if read.is_finished() {
                    Event::IN
                } else {
                    Event::empty()
                };
                write.union(read)
            }
        };
        return evs.intersection(other);
    }

    pub fn schedule_events(&mut self, evs: Event, qtoks: &mut Vec<demi::QToken>) {
        match &mut self.data {
            SocketData::Passive { accept } => {
                if evs.intersects(Event::IN) {
                    let tok = self.soc.accept().unwrap();
                    accept.start(tok, ());
                    qtoks.push(tok);
                }
            }
            SocketData::Active { write, read } => {
                if evs.intersects(Event::IN) {
                    let tok = self.soc.pop().unwrap();
                    read.start(tok, ());
                    qtoks.push(tok);
                }
                if evs.intersects(Event::OUT) {
                    if let Operation::Running { payload: _, tok } = write {
                        qtoks.push(*tok);
                    }
                }
            }
        };
    }

    pub fn process_event(&mut self, val: QResultValue) {
        match &mut self.data {
            SocketData::Passive { accept } => {
                if let QResultValue::Accept(acc) = val {
                    accept.complete(Ok(acc));
                } else {
                    panic!();
                }
            }

            SocketData::Active { write, read } => match val {
                QResultValue::Push => write.complete(Ok(())),
                QResultValue::Pop(sga) => read.complete(Ok(sga.into_iter())),
                _ => panic!(),
            },
        }
    }

    fn write_impl<F>(&mut self, func: F) -> PosixResult<usize>
    where
        F: FnOnce() -> demi::SgArray,
    {
        let write = match &mut self.data {
            SocketData::Active { write, read } => write,
            _ => return Err(PosixError::INVAL),
        };

        if write.poll() {
            write.get().unwrap();
        } else {
            return Err(PosixError::WOULDBLOCK);
        }

        let sga = func();
        let len = sga.len();
        write.start(self.soc.push(&sga).unwrap(), sga);
        return Ok(len);
    }

    fn read_impl<F>(&mut self, func: F) -> PosixResult<usize>
    where
        F: FnOnce(&mut demi::SgArrayByteIter) -> usize,
    {
        let read = match &mut self.data {
            SocketData::Active { write, read } => read,
            _ => return Err(PosixError::INVAL),
        };

        if !read.poll() {
            read.start(self.soc.pop().unwrap(), ());
            return Err(PosixError::WOULDBLOCK);
        }
        let iter = read.get_mut().unwrap();

        let len = func(iter);

        if iter.is_empty() {
            let _ = read.get();
            read.start(self.soc.pop().unwrap(), ());
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
