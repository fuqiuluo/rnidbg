use std::{mem, ptr};
use bitflags::bitflags;
use crate::android::dvm::DalvikVM64;
use crate::backend::Backend;

#[repr(C)]
#[derive(Default)]
pub struct Stat64 {
    pub st_dev: i64,
    pub st_ino: i64,
    pub st_mode: u32,//
    pub st_nlink: i32,//
    pub st_uid: i32,//
    pub st_gid: i32,//
    pub st_rdev: i64,
    pub __pad1: i64,
    pub st_size: usize,
    pub st_blksize: i32,//
    pub __pad2: i32,
    pub st_blocks: i64,

    pub st_atime: Timespec,
    pub st_mtime: Timespec,
    pub st_ctime: Timespec,

    pub __unused4: u32,
    pub __unused5: u32
}

#[repr(C)]
#[derive(Debug, Clone, Default)]
pub struct Timespec {
    pub tv_sec: i64,  // seconds
    pub tv_nsec: i64, // nanoseconds
}

#[repr(C)]
#[derive(Clone)]
pub struct Dirent {
    pub d_ino: u64,
    pub d_off: i64,
    pub d_reclen: u16,
    pub d_type: u8,
    pub d_name: [u8; 256]
}

bitflags!(
    /// Configuration options for opened files.
    #[derive(Debug, PartialEq, Clone, Copy)]
    pub struct OFlag: u32 {
        const O_ACCMODE = 0o0003;
        const O_RDONLY = 0o0;
        const O_WRONLY = 0o01;
        const O_RDWR = 0o02;
        const O_CREAT = 0o0100;
        const O_EXCL = 0o0200;
        const O_NOCTTY = 0o0400;
        const O_TRUNC = 0o01000;
        const O_APPEND = 0o02000;
        const O_NONBLOCK = 0o04000;
        const O_DSYNC = 0o010000;
        const O_DIRECT = 0o040000;
        const O_LARGEFILE = 0o0100000;
        const O_DIRECTORY = 0o0200000;
        const O_NOFOLLOW = 0o0400000;
        const O_NOATIME = 0o01000000;
        const O_CLOEXEC = 0o02000000;
        const __O_SYNC = 0o04000000;
        const O_SYNC = 0o04000000 | 0o010000;
        const O_PATH = 0o010000000;
    }
);

bitflags! {
    #[derive(Debug, PartialEq, Clone)]
    pub struct CloneFlag: u32 {
        const CLONE_VM = 0x100;
        const CLONE_FS = 0x200;
        const CLONE_FILES = 0x400;
        const CLONE_SIGHAND = 0x800;
        const CLONE_PTRACE = 0x2000;
        const CLONE_VFORK = 0x4000;
        const CLONE_PARENT = 0x8000;
        const CLONE_THREAD = 0x10000;
        const CLONE_NEWNS = 0x20000;
        const CLONE_SYSVSEM = 0x40000;
        const CLONE_SETTLS = 0x80000;
        const CLONE_PARENT_SETTID = 0x100000;
        const CLONE_CHILD_CLEARTID = 0x200000;
        const CLONE_DETACHED = 0x400000;
        const CLONE_UNTRACED = 0x800000;
        const CLONE_CHILD_SETTID = 0x01000000;
        const CLONE_NEWCGROUP = 0x02000000;
        const CLONE_NEWUTS = 0x04000000;
        const CLONE_NEWIPC = 0x08000000;
        const CLONE_NEWUSER = 0x10000000;
        const CLONE_NEWPID = 0x20000000;
        const CLONE_NEWNET = 0x40000000;
        const CLONE_IO = 0x80000000;
    }
}

#[repr(C)]
#[derive(Clone)]
pub struct Timeval {
    pub tv_sec: i64,
    pub tv_usec: i64
}

#[repr(C)]
#[derive(Clone)]
pub struct Timezone {
    pub tz_minuteswest: i32,
    pub tz_dsttime: i32
}

#[repr(C)]
#[derive(Clone)]
pub struct DlInfo {
    pub dli_fname: u64,
    pub dli_fbase: u64,
    pub dli_sname: u64,
    pub dli_saddr: u64
}

#[repr(C)]
#[derive(Clone)]
pub struct PropInfo {
    pub name: [u8; 32],
    pub serial: u32,
    pub value: [u8; 92]
}

pub mod thread {


}

pub mod socket {
    use bitflags::bitflags;

    #[derive(Debug, PartialEq, Clone)]
    #[repr(u32)]
    pub enum Pf {
        UNSPEC = 0,
        LOCAL = 1,
        INET = 2,
        AX25 = 3,
        IPX = 4,
        APPLETALK = 5,
        NETROM = 6,
        BRIDGE = 7,
        ATMPVC = 8,
        X25 = 9,
        INET6 = 10,
        ROSE = 11,
        DECnet = 12,
        NETBEUI = 13,
        SECURITY = 14,
        KEY = 15,
        NETLINK = 16,
        PACKET = 17,
        ASH = 18,
        ECONET = 19,
        ATMSVC = 20,
        RDS = 21,
        SNA = 22,
        IRDA = 23,
        PPPOX = 24,
        WANPIPE = 25,
        LLC = 26,
        IB = 27,
        MPLS = 28,
        CAN = 29,
        TIPC = 30,
        BLUETOOTH = 31,
        IUCV = 32,
        RXRPC = 33,
        ISDN = 34,
        PHONET = 35,
        IEEE802154 = 36,
        CAIF = 37,
        ALG = 38,
        NFC = 39,
        VSOCK = 40,
        KCM = 41,
        QIPCRTR = 42,
        SMC = 43,
        XDP = 44,
        MCTP = 45,
        MAX = 46
    }

    impl Pf {
        pub fn from_u32(value: u32) -> Self {
            match value {
                0 => Pf::UNSPEC,
                1 => Pf::LOCAL,
                2 => Pf::INET,
                3 => Pf::AX25,
                4 => Pf::IPX,
                5 => Pf::APPLETALK,
                6 => Pf::NETROM,
                7 => Pf::BRIDGE,
                8 => Pf::ATMPVC,
                9 => Pf::X25,
                10 => Pf::INET6,
                11 => Pf::ROSE,
                12 => Pf::DECnet,
                13 => Pf::NETBEUI,
                14 => Pf::SECURITY,
                15 => Pf::KEY,
                16 => Pf::NETLINK,
                17 => Pf::PACKET,
                18 => Pf::ASH,
                19 => Pf::ECONET,
                20 => Pf::ATMSVC,
                21 => Pf::RDS,
                22 => Pf::SNA,
                23 => Pf::IRDA,
                24 => Pf::PPPOX,
                25 => Pf::WANPIPE,
                26 => Pf::LLC,
                27 => Pf::IB,
                28 => Pf::MPLS,
                29 => Pf::CAN,
                30 => Pf::TIPC,
                31 => Pf::BLUETOOTH,
                32 => Pf::IUCV,
                33 => Pf::RXRPC,
                34 => Pf::ISDN,
                35 => Pf::PHONET,
                36 => Pf::IEEE802154,
                37 => Pf::CAIF,
                38 => Pf::ALG,
                39 => Pf::NFC,
                40 => Pf::VSOCK,
                41 => Pf::KCM,
                42 => Pf::QIPCRTR,
                43 => Pf::SMC,
                44 => Pf::XDP,
                45 => Pf::MCTP,
                46 => Pf::MAX,
                _ => Pf::UNSPEC
            }
        }
    }

    bitflags! {
        #[derive(Debug, PartialEq, Clone)]
        pub struct SockType: u32 {
            const SOCK_STREAM = 1;
            const SOCK_DGRAM = 2;
            const SOCK_RAW = 3;
            const SOCK_RDM = 4;
            const SOCK_SEQPACKET = 5;
            const SOCK_DCCP = 6;
            const SOCK_PACKET = 10;
            const SOCK_CLOEXEC = 02000000;
            const SOCK_NONBLOCK = 00004000;
        }
    }
}

pub mod prctl {
    use bitflags::bitflags;

    bitflags!(
        #[derive(Debug, PartialEq, Clone)]
        pub struct PrctlOp: u32 {
            const BIONIC_PR_SET_VMA = 0x53564d41;

            const UNKNOWN = 0xffffffff;
        }
    );
}

// https://github.com/thepowersgang/va_list-rs
#[repr(C)]
#[derive(Clone)]
pub struct CVaList<'a, T: Clone> {
    pub stack: u64,
    pub gr_top: u64,
    pub vr_top: u64,
    pub gr_offs: i32,
    pub vr_offs: i32,
    pub(crate) backend: Backend<'a, T>
}

impl<T: Clone> CVaList<'_, T> {
    pub(crate) unsafe fn get_gr<D>(&mut self) -> D {
        let pointer = (self.gr_top as usize - self.gr_offs.abs() as usize) as u64;
        let size = size_of::<D>();
        let rv = self.backend.mem_read_as_vec(pointer, size).unwrap();
        self.gr_offs += 8;
        let rv = rv.as_slice().as_ptr() as *const D;
        let rv = ptr::read(rv);
        rv
    }

    pub(crate) unsafe fn get_vr<D>(&mut self) -> D {
        let pointer = (self.vr_top as usize - self.vr_offs.abs() as usize) as u64;
        self.vr_offs += 16;
        let size = size_of::<D>();
        let rv = self.backend.mem_read_as_vec(pointer, size).unwrap();
        let rv = rv.as_slice().as_ptr() as *const D;
        let rv = ptr::read(rv);
        rv
    }
}

impl<D: Clone> CVaList<'_, D> {
    pub fn get<T: VaPrimitive<D>>(&mut self, dvm: &DalvikVM64<D>) -> T {
        unsafe { T::get(self, dvm) }
    }
}

pub trait VaPrimitive<T: Clone>: 'static {
    #[doc(hidden)]
    unsafe fn get(_: &mut CVaList<T>, dvm: &DalvikVM64<T>) -> Self;
}

macro_rules! impl_va_prim_gr {
    ($u: ty, $s: ty) => {
        impl<T: Clone> VaPrimitive<T> for $u {
            unsafe fn get(list: &mut CVaList<T>, dvm: &DalvikVM64<T>) -> Self {
                list.get_gr()
            }
        }
        impl<T: Clone> VaPrimitive<T> for $s {
            unsafe fn get(list: &mut CVaList<T>, dvm: &DalvikVM64<T>) -> Self {
                mem::transmute(<$u>::get(list, dvm))
            }
        }
    };
}

macro_rules! impl_va_prim_vr {
    ($($t:ty),+) => {
        $(
            impl<T: Clone> VaPrimitive<T> for $t {
                unsafe fn get(list: &mut CVaList<T>, dvm: &DalvikVM64<T>) -> Self {
                    list.get_vr()
                }
            }
        )+
    };
}

impl_va_prim_gr!{ usize, isize }
impl_va_prim_gr!{ u64, i64 }
impl_va_prim_gr!{ u32, i32 }
impl_va_prim_vr!{ f64, f32 }