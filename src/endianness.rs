use std::io;
use std::io::Read;
use std::marker::PhantomData;
use std::pin::Pin;
use bytemuck::Pod;

#[cfg(target_endian = "little")]
pub const IS_LITTLE_ENDIAN: bool = true;

#[cfg(target_endian = "big")]
pub const IS_LITTLE_ENDIAN: bool = false;

pub const IS_BIG_ENDIAN: bool = !IS_LITTLE_ENDIAN;

pub mod little_endian{
    use std::io;
    use std::io::Read;

    #[inline]
    pub fn read_u16(reader: &mut (impl Read + ?Sized)) -> io::Result<u16>{
        let mut data = [0u8; 2];

        reader.read_exact(&mut data)?;

        Ok(((data[0] as u16) << 8) | (data[1] as u16))
    }

    #[inline]
    pub fn read_u32(reader: &mut (impl Read + ?Sized)) -> io::Result<u32>{
        let mut data = [0u8; 4];

        reader.read_exact(&mut data)?;

        Ok(
            ((data[0] as u32) << 24) |
                ((data[1] as u32) << 16) |
                ((data[2] as u32) << 8) |
                (data[3] as u32)
        )
    }
}

pub struct StructMultiReadIter<'a, T: Pod + SwapEndian>{
    reader: &'a mut dyn Read,
    left_to_read: usize,
    swap_endian: bool,
    _phantom_data: PhantomData<&'static T>
}

impl<'a, T: Pod + SwapEndian> Iterator for StructMultiReadIter<'a, T>{
    type Item = io::Result<T>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.left_to_read == 0{
            None
        } else {
            Some(self.reader.read_struct(self.swap_endian))
        }
    }
}

impl<'a, T: Pod + SwapEndian> Drop for StructMultiReadIter<'a, T>{
    #[inline]
    fn drop(&mut self) {

        // read all the structs we would be reading and discard them to make the result after using 
        // this always be the same
        while let Some(_) = self.next() { }
    }
}




pub trait ReadExtensions: Read{
    #[inline]
    fn read_le_u16(&mut self) -> io::Result<u16>{
        little_endian::read_u16(self)
    }

    #[inline]
    fn read_le_u32(&mut self) -> io::Result<u32>{
        little_endian::read_u32(self)
    }

    #[inline]
    fn read_le_struct<T: Pod + SwapEndian>(&mut self) -> io::Result<T>{
        let mut data = T::zeroed();
        let bytes = bytemuck::bytes_of_mut(&mut data);

        self.read_exact(bytes)?;

        if cfg!(not(target_endian = "little")){
            data = data.swap_endian();
        }

        Ok(data)
    }

    #[inline]
    fn read_struct<T: Pod + SwapEndian>(&mut self, swap_endian: bool) -> io::Result<T>{
        let mut data = T::zeroed();
        let bytes = bytemuck::bytes_of_mut(&mut data);

        self.read_exact(bytes)?;

        if swap_endian{
            data = data.swap_endian();
        }

        Ok(data)
    }


    fn read_struct_multi<T: Pod + SwapEndian>(&mut self, swap_endian: bool, count: usize) -> io::Result<StructMultiReadIter<T>>;
}

impl<T: Read> ReadExtensions for T{
    // i was forced to put this here because it requires info about self
    #[inline]
    fn read_struct_multi<U: Pod + SwapEndian>(&mut self, swap_endian: bool, count: usize) -> io::Result<StructMultiReadIter<'_, U>>{
        Ok(StructMultiReadIter{
            reader: self,
            swap_endian,
            left_to_read: count,
            _phantom_data: Default::default()
        })
    }
}


pub trait SwapEndian: Clone + Copy{
    fn swap_endian(self) -> Self;
}

impl SwapEndian for u8{
    #[inline]
    fn swap_endian(self) -> Self {
        self
    }
}

impl SwapEndian for u16{
    #[inline]
    fn swap_endian(self) -> Self {
        self.swap_bytes()
    }
}
impl SwapEndian for u32{
    #[inline]
    fn swap_endian(self) -> Self {
        self.swap_bytes()
    }
}

impl SwapEndian for u64{
    #[inline]
    fn swap_endian(self) -> Self {
        self.swap_bytes()
    }
}

impl SwapEndian for i8{
    #[inline]
    fn swap_endian(self) -> Self {
        self
    }
}

impl SwapEndian for i16{
    #[inline]
    fn swap_endian(self) -> Self {
        self.swap_bytes()
    }
}
impl SwapEndian for i32{
    #[inline]
    fn swap_endian(self) -> Self {
        self.swap_bytes()
    }
}

impl SwapEndian for i64{
    #[inline]
    fn swap_endian(self) -> Self {
        self.swap_bytes()
    }
}

impl<T: SwapEndian, U: SwapEndian> SwapEndian for (T, U){
    #[inline]
    fn swap_endian(self) -> Self {
        (self.0.swap_endian(), self.1.swap_endian())
    }
}

impl<T: SwapEndian, U: SwapEndian, V: SwapEndian> SwapEndian for (T, U, V){
    #[inline]
    fn swap_endian(self) -> Self {
        (self.0.swap_endian(), self.1.swap_endian(), self.2.swap_endian())
    }
}

impl<T: SwapEndian, U: SwapEndian, V: SwapEndian, W: SwapEndian> SwapEndian for (T, U, V, W){
    #[inline]
    fn swap_endian(self) -> Self {
        (self.0.swap_endian(), self.1.swap_endian(), self.2.swap_endian(), self.3.swap_endian())
    }
}

impl<T: SwapEndian, const size: usize> SwapEndian for [T; size]{
    #[inline]
    fn swap_endian(mut self) -> Self {
        for elem in &mut self{
            *elem = elem.swap_endian();
        }

        self
    }
}