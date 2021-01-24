pub mod analog;
pub mod drive;
pub mod functions;
pub mod macros;

pub trait Bits<T>: Copy {
	fn bits(&self) -> T;
}
