pub mod address;
pub mod analog;
pub mod drive;
pub mod functions;
pub mod macros;

pub trait Bits<T>: Copy {
    fn bits(&self) -> T;
}

#[inline]
pub fn add_xor(buf: &mut [u8], len: usize) -> usize {
    let x = buf[0..len - 1].iter().fold(0, |acc, x| acc ^ x);
    buf[len - 1] = x;
    len
}
