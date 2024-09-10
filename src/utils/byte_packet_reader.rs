use std::string::FromUtf8Error;
use bytes::Buf;
use crate::utils::PacketFlag;

pub trait BytePacketReader: Buf {
    #[inline]
    fn get_str_with_flags(&mut self, packet_flag: PacketFlag) -> Result<String, FromUtf8Error>
    where Self: Sized
    {
        let buf = self.get_bytes_with_flags(packet_flag);
        String::from_utf8(buf)
    }

    #[inline]
    fn get_bytes_with_flags(&mut self, packet_flag: PacketFlag) -> Vec<u8>
    where Self: Sized
    {
        let mut tmp = 0;
        let len = if packet_flag.contains(PacketFlag::I16Len) {
            if packet_flag.contains(PacketFlag::ExtraLen) { tmp = 2; }
            self.get_i16() as usize
        } else if packet_flag.contains(PacketFlag::I32Len) {
            if packet_flag.contains(PacketFlag::ExtraLen) { tmp = 4; }
            self.get_i32() as usize
        } else if packet_flag.contains(PacketFlag::I64Len) {
            if packet_flag.contains(PacketFlag::ExtraLen) { tmp = 8; }
            self.get_i64() as usize
        } else {
            panic!("Invalid packet flag: {:?}", packet_flag);
        };
        let mut buf = vec![0u8; len - tmp];
        self.copy_to_slice(&mut buf);
        buf
    }
}

impl<T: Buf + ?Sized> BytePacketReader for &mut T {
}

impl<T: Buf + ?Sized> BytePacketReader for Box<T> {
}

impl BytePacketReader for &[u8] {
}