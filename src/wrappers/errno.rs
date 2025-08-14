use std::os::raw::c_int;
use thiserror::Error;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Error, PartialEq, Eq)]
#[repr(i32)]
pub enum PosixError {
    #[error("Operation not permitted")]
    PERM = 1,
    #[error("No such file or directory")]
    NOENT = 2,
    #[error("No such process")]
    SRCH = 3,
    #[error("Interrupted system call")]
    INTR = 4,
    #[error("Input/output error")]
    IO = 5,
    #[error("No such device or address")]
    NXIO = 6,
    #[error("Argument list too long")]
    TooBIG = 7,
    #[error("Exec format error")]
    NOEXEC = 8,
    #[error("Bad file descriptor")]
    BADF = 9,
    #[error("No child processes")]
    CHILD = 10,
    #[error("Cannot allocate memory")]
    NOMEM = 12,
    #[error("Permission denied")]
    ACCES = 13,
    #[error("Bad address")]
    FAULT = 14,
    #[error("Block device required")]
    NOTBLK = 15,
    #[error("Device or resource busy")]
    BUSY = 16,
    #[error("File exists")]
    EXIST = 17,
    #[error("Invalid cross-device link")]
    XDEV = 18,
    #[error("No such device")]
    NODEV = 19,
    #[error("Not a directory")]
    NOTDIR = 20,
    #[error("Is a directory")]
    ISDIR = 21,
    #[error("Invalid argument")]
    INVAL = 22,
    #[error("Too many open files in system")]
    NFILE = 23,
    #[error("Too many open files")]
    MFILE = 24,
    #[error("Inappropriate ioctl for device")]
    NOTTY = 25,
    #[error("Text file busy")]
    TXTBSY = 26,
    #[error("File too large")]
    FBIG = 27,
    #[error("No space left on device")]
    NOSPC = 28,
    #[error("Illegal seek")]
    SPIPE = 29,
    #[error("Read-only file system")]
    ROFS = 30,
    #[error("Too many links")]
    MLINK = 31,
    #[error("Broken pipe")]
    PIPE = 32,
    #[error("Numerical argument out of domain")]
    DOM = 33,
    #[error("Numerical result out of range")]
    RANGE = 34,
    #[error("File name too long")]
    NAMETOOLONG = 36,
    #[error("No locks available")]
    NOLCK = 37,
    #[error("Function not implemented")]
    NOSYS = 38,
    #[error("Directory not empty")]
    NOTEMPTY = 39,
    #[error("Too many levels of symbolic links")]
    LOOP = 40,
    #[error("Resource temporarily unavailable")]
    WOULDBLOCK = 11,
    #[error("No message of desired type")]
    NOMSG = 42,
    #[error("Identifier removed")]
    IDRM = 43,
    #[error("Channel number out of range")]
    CHRNG = 44,
    #[error("Level 2 not synchronised")]
    L2NSYNC = 45,
    #[error("Level 3 halted")]
    L3HLT = 46,
    #[error("Level 3 reset")]
    L3RST = 47,
    #[error("Link number out of range")]
    LNRNG = 48,
    #[error("Protocol driver not attached")]
    UNATCH = 49,
    #[error("No CSI structure available")]
    NOCSI = 50,
    #[error("Level 2 halted")]
    L2HLT = 51,
    #[error("Invalid exchange")]
    BADE = 52,
    #[error("Invalid request descriptor")]
    BADR = 53,
    #[error("Exchange full")]
    XFULL = 54,
    #[error("No anode")]
    NOANO = 55,
    #[error("Invalid request code")]
    BADRQC = 56,
    #[error("Invalid slot")]
    BADSLT = 57,
    #[error("Resource deadlock avoided")]
    DEADLOCK = 35,
    #[error("Bad font file format")]
    BFONT = 59,
    #[error("Device not a stream")]
    NOSTR = 60,
    #[error("No data available")]
    NODATA = 61,
    #[error("Timer expired")]
    TIME = 62,
    #[error("Out of streams resources")]
    NOSR = 63,
    #[error("Machine is not on the network")]
    NONET = 64,
    #[error("Package not installed")]
    NOPKG = 65,
    #[error("Object is remote")]
    REMOTE = 66,
    #[error("Link has been severed")]
    NOLINK = 67,
    #[error("Advertise error")]
    ADV = 68,
    #[error("Srmount error")]
    SRMNT = 69,
    #[error("Communication error on send")]
    COMM = 70,
    #[error("Protocol error")]
    PROTO = 71,
    #[error("Multihop attempted")]
    MULTIHOP = 72,
    #[error("RFS specific error")]
    DOTDOT = 73,
    #[error("Bad message")]
    BADMSG = 74,
    #[error("Value too large for defined data type")]
    OVERFLOW = 75,
    #[error("Name not unique on network")]
    NOTUNIQ = 76,
    #[error("File descriptor in bad state")]
    BADFD = 77,
    #[error("Remote address changed")]
    REMCHG = 78,
    #[error("Can not access a needed shared library")]
    LIBACC = 79,
    #[error("Accessing a corrupted shared library")]
    LIBBAD = 80,
    #[error(".lib section in a.out corrupted")]
    LIBSCN = 81,
    #[error("Attempting to link in too many shared libraries")]
    LIBMAX = 82,
    #[error("Cannot exec a shared library directly")]
    LIBEXEC = 83,
    #[error("Invalid or incomplete multibyte or wide character")]
    ILSEQ = 84,
    #[error("Interrupted system call should be restarted")]
    RESTART = 85,
    #[error("Streams pipe error")]
    STRPIPE = 86,
    #[error("Too many users")]
    USERS = 87,
    #[error("Socket operation on non-socket")]
    NOTSOCK = 88,
    #[error("Destination address required")]
    DESTADDRREQ = 89,
    #[error("Message too long")]
    MSGSIZE = 90,
    #[error("Protocol wrong type for socket")]
    PROTOTYPE = 91,
    #[error("Protocol not available")]
    NOPROTOOPT = 92,
    #[error("Protocol not supported")]
    PROTONOSUPPORT = 93,
    #[error("Socket type not supported")]
    SOCKTNOSUPPORT = 94,
    #[error("Operation not supported")]
    OPNOTSUPP = 95,
    #[error("Protocol family not supported")]
    PFNOSUPPORT = 96,
    #[error("Address family not supported by protocol")]
    AFNOSUPPORT = 97,
    #[error("Address already in use")]
    ADDRINUSE = 98,
    #[error("Cannot assign requested address")]
    ADDRNOTAVAIL = 99,
    #[error("Network is down")]
    NETDOWN = 100,
    #[error("Network is unreachable")]
    NETUNREACH = 101,
    #[error("Network dropped connection on reset")]
    NETRESET = 102,
    #[error("Software caused connection abort")]
    CONNABORTED = 103,
    #[error("Connection reset by peer")]
    CONNRESET = 104,
    #[error("No buffer space available")]
    NOBUFS = 105,
    #[error("Transport endpoint is already connected")]
    ISCONN = 106,
    #[error("Transport endpoint is not connected")]
    NOTCONN = 107,
    #[error("Cannot send after transport endpoint shutdown")]
    SHUTDOWN = 108,
    #[error("Too many references: cannot splice")]
    TOOMANYREFS = 109,
    #[error("Connection timed out")]
    TIMEDOUT = 110,
    #[error("Connection refused")]
    CONNREFUSED = 111,
    #[error("Host is down")]
    HOSTDOWN = 112,
    #[error("No route to host")]
    HOSTUNREACH = 113,
    #[error("Operation already in progress")]
    ALREADY = 114,
    #[error("Operation now in progress")]
    INPROGRESS = 115,
    #[error("Stale file handle")]
    STALE = 116,
    #[error("Structure needs cleaning")]
    UCLEAN = 117,
    #[error("Not a XENIX named type file")]
    NOTNAM = 118,
    #[error("No XENIX semaphores available")]
    NAVAIL = 119,
    #[error("Is a named type file")]
    ISNAM = 120,
    #[error("Remote I/O error")]
    REMOTEIO = 121,
    #[error("Disk quota exceeded")]
    DQUOT = 122,
    #[error("No medium found")]
    NOMEDIUM = 123,
    #[error("Wrong medium type")]
    MEDIUMTYPE = 124,
    #[error("Operation cancelled")]
    CANCELED = 125,
    #[error("Required key not available")]
    NOKEY = 126,
    #[error("Key has expired")]
    KEYEXPIRED = 127,
    #[error("Key has been revoked")]
    KEYREVOKED = 128,
    #[error("Key was rejected by service")]
    KEYREJECTED = 129,
    #[error("Owner died")]
    OWNERDEAD = 130,
    #[error("State not recoverable")]
    NOTRECOVERABLE = 131,
    #[error("Operation not possible due to RF-kill")]
    RFKILL = 132,
    #[error("Memory page has hardware error")]
    HWPOISON = 133,
}

impl PosixError {
    pub fn from_errno() -> PosixResult<()> {
        let err = unsafe { libc::__errno_location().read() };
        return Self::from_error_code(err);
    }

    /// returns Ok(()) if errno == 0
    ///
    /// panics if errno does not map to anything
    #[allow(unreachable_code)]
    pub fn from_error_code(code: c_int) -> PosixResult<()> {
        if code == 0 {
            return Ok(());
        } else if code <= 133 {
            let var: PosixError = unsafe { std::mem::transmute(code) };
            return Err(var);
        } else {
            panic!("invalid errno: {}\n", code);
        };
    }
}

impl std::convert::Into<c_int> for PosixError {
    fn into(self) -> c_int {
        return self as c_int;
    }
}

pub type PosixResult<T> = Result<T, PosixError>;
