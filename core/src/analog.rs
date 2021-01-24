use num_derive::{FromPrimitive, ToPrimitive};

#[derive(Clone, Debug, PartialEq, FromPrimitive, ToPrimitive)]
pub enum AnalogNumber {
	A0 = 0,
	A1,
	A2,
	A3,
	A4,
	A5,
	A6,
	A7,
}
