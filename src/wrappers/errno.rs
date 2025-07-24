use std::os::raw::c_int;
use thiserror::Error;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Error)]
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
    /// returns Ok(()) if errno == 0
    ///
    /// panics if errno does not map to anything
    #[allow(unreachable_code)]
    pub fn from_errno(errno: c_int) -> Result<(), Self> {
        use PosixError::*;

        if errno == 0 {
            return Ok(());
        } else if errno <= 133 {
            let var: PosixError = unsafe { std::mem::transmute(errno) };
            return Err(var);
        } else {
            panic!("invalid errno: {}\n", errno);
        };
        return match errno {
            0 => Ok(()),
            1 => Err(PERM),
            2 => Err(NOENT),
            3 => Err(SRCH),
            4 => Err(INTR),
            5 => Err(IO),
            6 => Err(NXIO),
            7 => Err(TooBIG),
            8 => Err(NOEXEC),
            9 => Err(BADF),
            10 => Err(CHILD),
            12 => Err(NOMEM),
            13 => Err(ACCES),
            14 => Err(FAULT),
            15 => Err(NOTBLK),
            16 => Err(BUSY),
            17 => Err(EXIST),
            18 => Err(XDEV),
            19 => Err(NODEV),
            20 => Err(NOTDIR),
            21 => Err(ISDIR),
            22 => Err(INVAL),
            23 => Err(NFILE),
            24 => Err(MFILE),
            25 => Err(NOTTY),
            26 => Err(TXTBSY),
            27 => Err(FBIG),
            28 => Err(NOSPC),
            29 => Err(SPIPE),
            30 => Err(ROFS),
            31 => Err(MLINK),
            32 => Err(PIPE),
            33 => Err(DOM),
            34 => Err(RANGE),
            36 => Err(NAMETOOLONG),
            37 => Err(NOLCK),
            38 => Err(NOSYS),
            39 => Err(NOTEMPTY),
            40 => Err(LOOP),
            11 => Err(WOULDBLOCK),
            42 => Err(NOMSG),
            43 => Err(IDRM),
            44 => Err(CHRNG),
            45 => Err(L2NSYNC),
            46 => Err(L3HLT),
            47 => Err(L3RST),
            48 => Err(LNRNG),
            49 => Err(UNATCH),
            50 => Err(NOCSI),
            51 => Err(L2HLT),
            52 => Err(BADE),
            53 => Err(BADR),
            54 => Err(XFULL),
            55 => Err(NOANO),
            56 => Err(BADRQC),
            57 => Err(BADSLT),
            35 => Err(DEADLOCK),
            59 => Err(BFONT),
            60 => Err(NOSTR),
            61 => Err(NODATA),
            62 => Err(TIME),
            63 => Err(NOSR),
            64 => Err(NONET),
            65 => Err(NOPKG),
            66 => Err(REMOTE),
            67 => Err(NOLINK),
            68 => Err(ADV),
            69 => Err(SRMNT),
            70 => Err(COMM),
            71 => Err(PROTO),
            72 => Err(MULTIHOP),
            73 => Err(DOTDOT),
            74 => Err(BADMSG),
            75 => Err(OVERFLOW),
            76 => Err(NOTUNIQ),
            77 => Err(BADFD),
            78 => Err(REMCHG),
            79 => Err(LIBACC),
            80 => Err(LIBBAD),
            81 => Err(LIBSCN),
            82 => Err(LIBMAX),
            83 => Err(LIBEXEC),
            84 => Err(ILSEQ),
            85 => Err(RESTART),
            86 => Err(STRPIPE),
            87 => Err(USERS),
            88 => Err(NOTSOCK),
            89 => Err(DESTADDRREQ),
            90 => Err(MSGSIZE),
            91 => Err(PROTOTYPE),
            92 => Err(NOPROTOOPT),
            93 => Err(PROTONOSUPPORT),
            94 => Err(SOCKTNOSUPPORT),
            95 => Err(OPNOTSUPP),
            96 => Err(PFNOSUPPORT),
            97 => Err(AFNOSUPPORT),
            98 => Err(ADDRINUSE),
            99 => Err(ADDRNOTAVAIL),
            100 => Err(NETDOWN),
            101 => Err(NETUNREACH),
            102 => Err(NETRESET),
            103 => Err(CONNABORTED),
            104 => Err(CONNRESET),
            105 => Err(NOBUFS),
            106 => Err(ISCONN),
            107 => Err(NOTCONN),
            108 => Err(SHUTDOWN),
            109 => Err(TOOMANYREFS),
            110 => Err(TIMEDOUT),
            111 => Err(CONNREFUSED),
            112 => Err(HOSTDOWN),
            113 => Err(HOSTUNREACH),
            114 => Err(ALREADY),
            115 => Err(INPROGRESS),
            116 => Err(STALE),
            117 => Err(UCLEAN),
            118 => Err(NOTNAM),
            119 => Err(NAVAIL),
            120 => Err(ISNAM),
            121 => Err(REMOTEIO),
            122 => Err(DQUOT),
            123 => Err(NOMEDIUM),
            124 => Err(MEDIUMTYPE),
            125 => Err(CANCELED),
            126 => Err(NOKEY),
            127 => Err(KEYEXPIRED),
            128 => Err(KEYREVOKED),
            129 => Err(KEYREJECTED),
            130 => Err(OWNERDEAD),
            131 => Err(NOTRECOVERABLE),
            132 => Err(RFKILL),
            133 => Err(HWPOISON),
            _ => panic!("invalid errno: {}", errno),
        };
    }
}

impl std::convert::Into<c_int> for PosixError {
    fn into(self) -> c_int {
        return self as c_int;
    }
}

pub type PosixResult<T> = Result<T, PosixError>;
