use std::mem::MaybeUninit;

use libc::{SIG_SETMASK, pthread_sigmask, sigset_t};

pub struct Sigset {
    old: Option<MaybeUninit<sigset_t>>,
}

impl Sigset {
    pub fn mask(new: *const sigset_t) -> Self {
        let new = unsafe { new.as_ref() };
        let new = match new {
            None => return Self { old: None },
            Some(set) => set,
        };

        let mut old = MaybeUninit::uninit();
        unsafe {
            assert_eq!(pthread_sigmask(SIG_SETMASK, new, old.as_mut_ptr()), 0);
        }

        return Self { old: Some(old) };
    }
}

impl Drop for Sigset {
    fn drop(&mut self) {
        match self.old {
            None => return,
            Some(old) => unsafe {
                assert_eq!(
                    pthread_sigmask(SIG_SETMASK, old.as_ptr(), std::ptr::null_mut()),
                    0
                );
            },
        }
    }
}
