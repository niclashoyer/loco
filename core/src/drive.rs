#[derive(Clone, Debug, PartialEq)]
pub enum Direction {
	Forward,
	Backward,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Speed {
	Stop,
	EmergencyStop,
	Steps14(u8),
	Steps28(u8),
	Steps128(u8),
}
