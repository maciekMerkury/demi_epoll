use std::{fmt::Debug, mem::{self}, time::Duration};

use crate::wrappers::{demi::{self, QResult, QToken}, errno::{PosixError, PosixResult}};

pub trait Schedulable: Sized {
    fn from_qresult(result: QResult) -> Self;
}

impl Schedulable for demi::AcceptResult {
    fn from_qresult(result: QResult) -> Self {
        let val = result.value.unwrap();
        if let demi::QResultValue::Accept(accept_res) = val {
            return accept_res;
        } else {
            panic!("cannot create AcceptResult from {:?}", val);
        }
    }
}

impl Schedulable for () {
    fn from_qresult(val: QResult) -> Self {
        assert!(matches!(val.value.unwrap(), demi::QResultValue::Push(_)));
    }
}

impl Schedulable for demi::SgArrayByteIter {
    fn from_qresult(result: QResult) -> Self {
        let val = result.value.unwrap();
        if let demi::QResultValue::Pop(buf) = val{
            return buf.into_iter();
        } else {
            panic!("cannot create SgArrayByteIter from {:?}", val);
        }
    }
}

/// takes ownership of payload P, which will be dropped in transition to Completed
#[derive(Debug)]
pub enum Operation<P, T>
where P: Debug,
      T: Schedulable + Debug
{
    None,
    Running {
        payload: P,
        tok: QToken,
    },
    Completed(PosixResult<T>),
}

impl<P, T> Operation<P, T>
where P: Debug,
      T: Schedulable + Debug
{
    pub const fn default() -> Self {
        return Self::None;
    }

    pub fn new(tok: demi::QToken, payload: P) -> Self {
        return Self::Running { payload, tok };
    }

    pub fn schedule(&mut self, tok: demi::QToken, payload: P) {
        assert!(matches!(self, Operation::None));

        *self = Self::new(tok, payload);
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

    pub fn is_finished(&self) -> bool {
        return matches!(self, Self::Completed(_));
    }
    pub fn is_running(&self) -> bool {
        return matches!(self, Self::Running { payload, tok });
    }

    pub fn get_or_schedule<F>(&mut self, func: F) -> Option<PosixResult<T>>
        where F: FnOnce() -> (demi::QToken, P)
    {
        use Operation as Op;

        match self {
            Op::None => {
                let (tok, payload) = func();
                *self = Op::Running { payload, tok };
                return None;
            },
            Op::Running{..} => if self.poll() {
                                    return Some(self.get());
                                } else {
                                    return None;
                                },
            Op::Completed(_) => return Some(self.get()),
        }
    }

    fn wait(&mut self, timeout: Option<Duration>) {
        let tok = if let Self::Running { tok, .. } = self {
            *tok
        } else {
            return;
        };

        let res = match demi::wait(tok, timeout) {
            Ok(res) => Some(Ok(res)),
            Err(err) => if err == PosixError::WOULDBLOCK { None } else { Some(Err(err)) }
        };

        if let Some(res) = res {
            *self = Self::Completed(res.map(T::from_qresult));
        }
    }

    #[inline]
    pub fn poll(&mut self) -> bool {
        self.wait(Some(Duration::ZERO));
        return self.is_finished();
    }

    #[inline]
    pub fn block(&mut self) {
        self.wait(None);
    }
}

