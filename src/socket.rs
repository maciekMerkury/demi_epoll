use std::mem::MaybeUninit;
use std::usize;

use log::trace;

use crate::dpoll::Event;
use crate::operation::Operation;

use crate::wrappers::demi::QResultValue;
use crate::wrappers::errno::PosixError;
use crate::wrappers::{demi, errno::PosixResult};

#[derive(Debug)]
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

    #[allow(dead_code)]
    pub fn flush(&mut self) {
        match self {
            SocketData::Passive { accept } => accept.block(),
            SocketData::Active { write, read } => {
                write.block();
                read.block();
            }
        }
    }
}

#[derive(Debug)]
enum OpenStatus {
    Open,
    Closed,
    Error(PosixError),
}

#[derive(Debug)]
pub struct Socket {
    pub soc: demi::SocketQd,
    /// to be used with getsockname
    pub addr: Option<libc::sockaddr_in>,

    status: OpenStatus,
    data: SocketData,
}

impl Socket {
    pub fn socket() -> PosixResult<Self> {
        return demi::SocketQd::new().map(Self::new);
    }

    pub fn new(soc: demi::SocketQd) -> Self {
        return Self {
            soc,
            addr: None,
            status: OpenStatus::Open,
            data: SocketData::Passive {
                accept: Operation::None,
            },
        };
    }

    pub fn is_open(&self) -> bool {
        return matches!(self.status, OpenStatus::Open);
    }

    fn status(&self) -> PosixResult<bool> {
        return match self.status {
            OpenStatus::Open => Ok(true),
            OpenStatus::Closed => Ok(false),
            OpenStatus::Error(err) => Err(err),
        };
    }

    #[inline]
    pub fn bind(&mut self, addr: &libc::sockaddr_in) -> PosixResult<()> {
        assert!(self.status()?);
        self.soc.bind(addr)?;
        self.data = SocketData::new_passive();
        self.addr = Some(*addr);

        return Ok(());
    }

    #[inline]
    pub fn listen(&mut self, backlog: i32) -> PosixResult<()> {
        assert!(self.status()?);
        return self.soc.listen(backlog);
    }

    pub fn accept(
        &mut self,
        addr: Option<&mut MaybeUninit<libc::sockaddr_in>>,
    ) -> PosixResult<Self> {
        assert!(self.status()?);
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
        assert!(self.status()?);
        trace!("writing {} to {}", src.len(), self.soc.qd);
        let res = self.write_impl(|| demi::SgArray::from_slice(src));
        trace!("res: {res:?}, BRUH: {self:?}");
        return res;
    }

    pub fn writev(&mut self, src: &[libc::iovec]) -> PosixResult<usize> {
        assert!(self.status()?);
        return self.write_impl(|| demi::SgArray::from_slices(src));
    }

    pub fn read(&mut self, dst: &mut [MaybeUninit<u8>]) -> PosixResult<usize> {
        assert!(self.status()?);
        return self.read_impl(|it| it.copy_bytes(dst));
    }

    pub fn readv(&mut self, dst: &mut [libc::iovec]) -> PosixResult<usize> {
        assert!(self.status()?);
        return self.read_impl(|it| it.copy_into_iovecs(dst));
    }

    pub fn close(&mut self) {
        assert!(self.status().unwrap());
        //self.data.flush();
        self.soc.close().unwrap();
        self.status = OpenStatus::Closed;
    }

    pub fn close_with_err(&mut self, err: PosixError) {
        self.soc.close().unwrap();
        self.status = OpenStatus::Error(err)
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
                    let tok = match accept {
                        Operation::None => {
                            let tok = self.soc.accept().unwrap();
                            accept.start(tok, ());
                            tok
                        }
                        Operation::Running { tok, .. } => *tok,
                        Operation::Completed(_) => unreachable!(),
                    };
                    qtoks.push(tok);
                }
            }
            SocketData::Active { write, read } => {
                if evs.intersects(Event::IN) {
                    let tok = match read {
                        Operation::Running { tok, .. } => *tok,
                        Operation::None => {
                            let tok = self.soc.pop().unwrap();
                            read.start(tok, ());
                            tok
                        }
                        Operation::Completed(_) => unreachable!(),
                    };
                    qtoks.push(tok);
                }

                // always schedule pending writes
                match write {
                    Operation::Running { tok, .. } => qtoks.push(*tok),
                    _ if evs.intersects(Event::OUT) => unreachable!(),
                    _ => {}
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
            SocketData::Active { write, .. } => write,
            _ => return Err(PosixError::INVAL),
        };

        if !write.is_none() {
            if write.poll() {
                write.get().unwrap();
            } else {
                return Err(PosixError::WOULDBLOCK);
            }
        }

        let sga = func();
        let len = sga.len();
        write.start(self.soc.push(&sga).unwrap(), sga);
        return Ok(len);
    }

    fn read_impl<F>(&mut self, func: F) -> PosixResult<usize>
    where
        F: FnOnce(&mut demi::SgArrayByteIter) -> Option<usize>,
    {
        let read = match &mut self.data {
            SocketData::Active { read, .. } => read,
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

        trace!("read {:?} bytes", len);
        return len.ok_or(PosixError::WOULDBLOCK);
    }
}

impl std::convert::From<demi::AcceptResult> for Socket {
    fn from(value: demi::AcceptResult) -> Self {
        return Self {
            soc: value.qd,
            addr: Some(value.addr),
            status: OpenStatus::Open,
            data: SocketData::new_active(),
        };
    }
}
