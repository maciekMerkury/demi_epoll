use std::{
    fmt::Debug,
    mem::{self},
    time::Duration,
};

use log::trace;

use crate::wrappers::{
    demi::{self, QResult, QToken},
    errno::{PosixError, PosixResult},
};

pub trait Schedulable: Sized {
    type Payload: Debug;

    fn from_qresult(result: QResult) -> Self;

    fn schedule(soc: &mut demi::SocketQd, payload: &mut Self::Payload) -> demi::QToken;
}

impl Schedulable for demi::AcceptResult {
    type Payload = ();

    fn from_qresult(result: QResult) -> Self {
        let val = result.value.unwrap();
        if let demi::QResultValue::Accept(accept_res) = val {
            return accept_res;
        } else {
            panic!("cannot create AcceptResult from {:?}", val);
        }
    }

    fn schedule(soc: &mut demi::SocketQd, _: &mut Self::Payload) -> demi::QToken {
        return soc.accept().unwrap();
    }
}

impl Schedulable for () {
    type Payload = demi::SgArray;

    fn from_qresult(val: QResult) -> Self {
        assert!(matches!(val.value.unwrap(), demi::QResultValue::Push));
    }

    fn schedule(soc: &mut demi::SocketQd, sga: &mut Self::Payload) -> demi::QToken {
        return soc.push(&sga).unwrap();
    }
}

impl Schedulable for demi::SgArrayByteIter {
    type Payload = ();

    fn from_qresult(result: QResult) -> Self {
        let val = result.value.unwrap();
        if let demi::QResultValue::Pop(buf) = val {
            return buf.into_iter();
        } else {
            panic!("cannot create SgArrayByteIter from {:?}", val);
        }
    }

    fn schedule(soc: &mut demi::SocketQd, _: &mut Self::Payload) -> demi::QToken {
        return soc.pop().unwrap();
    }
}

/// takes ownership of payload P, which will be dropped in transition to Completed
#[derive(Debug)]
pub enum Operation<T>
where
    T: Schedulable + Debug,
{
    None,
    Running { _payload: T::Payload, tok: QToken },
    Completed(PosixResult<T>),
}

impl<T> Operation<T>
where
    T: Schedulable + Debug,
{
    pub const fn default() -> Self {
        return Self::None;
    }

    pub fn start(&mut self, tok: demi::QToken, payload: T::Payload) {
        assert!(matches!(self, Operation::None));

        *self = Self::Running {
            _payload: payload,
            tok,
        };
    }

    pub fn complete(&mut self, result: PosixResult<T>) {
        assert!(self.is_running());
        *self = Self::Completed(result);
    }

    pub fn get(&mut self) -> PosixResult<T> {
        match mem::replace(self, Operation::None) {
            Operation::Completed(res) => return res,
            other => panic!("cannot get a {:?}", other),
        }
    }

    pub fn get_mut(&mut self) -> PosixResult<&mut T> {
        match self {
            Operation::Completed(res) => return res.as_mut().map_err(|e| *e),
            other => panic!("cannot get a {:?}", other),
        };
    }

    #[allow(dead_code)]
    pub fn get_mut_or_schedule<'a, F>(&'a mut self, func: F) -> Option<PosixResult<&'a mut T>>
    where
        F: FnOnce() -> (&'a mut demi::SocketQd, T::Payload),
    {
        use Operation as Op;

        match self {
            Op::None => {
                let (soc, mut payload) = func();
                let tok = T::schedule(soc, &mut payload);
                *self = Op::Running {
                    _payload: payload,
                    tok,
                };
                return None;
            }
            Op::Running { .. } => {
                if self.poll() {
                    return Some(self.get_mut());
                } else {
                    return None;
                }
            }
            Op::Completed(_) => return Some(self.get_mut()),
        }
    }

    pub fn get_or_schedule<'a, F>(&'a mut self, func: F) -> Option<PosixResult<T>>
    where
        F: FnOnce() -> (&'a mut demi::SocketQd, T::Payload),
    {
        use Operation as Op;

        match self {
            Op::None => {
                let (soc, mut payload) = func();
                let tok = T::schedule(soc, &mut payload);
                *self = Op::Running {
                    _payload: payload,
                    tok,
                };
                return None;
            }
            Op::Running { .. } => {
                if self.poll() {
                    return Some(self.get());
                } else {
                    return None;
                }
            }
            Op::Completed(_) => return Some(self.get()),
        }
    }

    pub fn is_finished(&self) -> bool {
        return matches!(self, Self::Completed(_));
    }

    pub fn is_running(&self) -> bool {
        return matches!(self, Self::Running { .. });
    }

    pub fn is_none(&self) -> bool {
        return matches!(self, Self::None);
    }

    #[inline]
    pub fn poll(&mut self) -> bool {
        trace!("polling {:?}", self);
        self.wait(Some(Duration::ZERO));
        return self.is_none() || self.is_finished();
    }

    #[inline]
    pub fn block(&mut self) {
        self.wait(None);
    }

    fn wait(&mut self, timeout: Option<Duration>) {
        let tok = if let Self::Running { tok, .. } = self {
            *tok
        } else {
            return;
        };

        let res = match demi::wait(tok, timeout) {
            Ok(res) => Some(Ok(res)),
            Err(err) => {
                if err == PosixError::TIMEDOUT {
                    None
                } else {
                    panic!("{}", err);
                }
            }
        };

        if let Some(res) = res {
            *self = Self::Completed(res.map(T::from_qresult));
        }
    }
}
