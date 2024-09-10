use bytes::{BufMut, BytesMut};
use crate::utils::PacketFlag;

pub trait BytePacketBuilder: BufMut {
    #[inline]
    fn put_packet(&mut self, builder: &mut dyn FnMut(&mut BytesMut))
    where
        Self: Sized,
    {
        let mut buf = BytesMut::new();
        builder(&mut buf);
        self.put(buf);
    }

    #[inline]
    fn put_packet_with_flags(&mut self, builder: &mut dyn FnMut(&mut BytesMut), flag: PacketFlag)
    where
        Self: Sized,
    {
        let mut buf = BytesMut::new();
        builder(&mut buf);
        self.put_bytes_with_flags(buf.as_ref(), flag);
    }

    #[inline]
    fn put_bytes_with_flags(&mut self, bytes: &[u8], flag: PacketFlag)
    where
        Self: Sized,
    {
        let mut len = bytes.len();
        if flag.contains(PacketFlag::I16Len) {
            if flag.contains(PacketFlag::ExtraLen) {
                len += 2;
            }
            self.put_i16(len as i16);
        } else if flag.contains(PacketFlag::I32Len) {
            if flag.contains(PacketFlag::ExtraLen) {
                len += 4;
            }
            self.put_i32(len as i32);
        } else if flag.contains(PacketFlag::I64Len) {
            if flag.contains(PacketFlag::ExtraLen) {
                len += 8;
            }
            self.put_i64(len as i64);
        }
        self.put_slice(bytes);
    }
}

impl BytePacketBuilder for BytesMut {}
impl<T: BufMut + ?Sized> BytePacketBuilder for &mut T {}
impl<T: BufMut + ?Sized> BytePacketBuilder for Box<T> {}
impl BytePacketBuilder for Vec<u8> {}