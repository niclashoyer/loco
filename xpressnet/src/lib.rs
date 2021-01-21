use bitflags::bitflags;
use heapless::consts::U17;
use heapless::Vec;

pub trait Bits<T>: Copy {
	fn bits(&self) -> T;
}

bitflags! {
	pub struct CentralState: u8 {
		const EMERGENCY_OFF = 0b0000_0001;
		const EMERGENCY_STOP = 0b0000_0010;
		const AUTOMATIC_START = 0b0000_0100;
		const SERVICE_MODE = 0b0000_1000;
		const POWER_UP = 0b0100_0000;
		const RAM_ERROR = 0b1000_0000;
	}
}

impl Bits<u8> for CentralState {
	fn bits(&self) -> u8 {
		self.bits
	}
}

#[derive(Debug)]
pub struct MessageBytes {
	bytes: Vec<u8, U17>,
}

impl MessageBytes {
	fn from_slice(slice: &[u8]) -> Self {
		Self {
			bytes: Vec::<_, _>::from_slice(slice).unwrap(),
		}
	}

	fn xor(&self) -> u8 {
		let mut iter = self.bytes[0..].iter();
		let mut xor = *iter.next().unwrap();
		for byte in iter {
			xor = xor ^ byte;
		}
		xor
	}
}

#[derive(Debug)]
pub enum CentralMessage<S: Bits<u8>> {
	TrackPowerOn,
	TrackPowerOff,
	EmergencyStop,
	//FeedbackBroadcast([(u8, u8); 7])
	Version(u8, u8),
	State(S),
	TransferError,
	StationBusy,
	UnknownCommand,
}

#[derive(Debug)]
pub enum Error {
	ParseError,
}

macro_rules! xor {
	( [$x:expr, $( $y:expr ),*] ) => {
		[$x,$( $y ),*,$x $( ^$y )*]
	}
}

macro_rules! mb {
	( $b:ident[$p:expr] <= $($x:tt)* ) => {{
		$b[$p].copy_from_slice($($x)*);
		$b[$p].len()
	}};
}

impl<S: Bits<u8>> CentralMessage<S> {
	pub fn to_buf(&self, buf: &mut [u8]) -> usize {
		use CentralMessage::*;
		match self {
			&TrackPowerOn => mb!(buf[0..3] <= &xor!([0x61, 0x01])),
			&TrackPowerOff => mb!(buf[0..3] <= &xor!([0x61, 0x00])),
			&EmergencyStop => mb!(buf[0..3] <= &xor!([0x81, 0x00])),
			&Version(u, l) => mb!(buf[0..5] <= &xor!([0x63, 0x21, u, l])),
			&State(state) => mb!(buf[0..4] <= &xor!([0x62, 0x22, state.bits()])),
			&TransferError => mb!(buf[0..3] <= &xor!([0x61, 0x80])),
			&StationBusy => mb!(buf[0..3] <= &xor!([0x61, 0x81])),
			&UnknownCommand => mb!(buf[0..3] <= &xor!([0x61, 0x82])),
		}
	}
}

#[derive(Debug)]
pub enum DeviceMessage {
	GetVersion,
	GetState,
}

impl DeviceMessage {
	pub fn from_bytes(bytes: &[u8]) -> Result<DeviceMessage, Error> {
		use DeviceMessage::*;
		match bytes {
			&[0x21, 0x21, 0x00, ..] => Ok(GetVersion),
			&[0x21, 0x24, 0x05, ..] => Ok(GetState),
			_ => {
				println!("X: {:#04X?}", bytes);
				Err(Error::ParseError)
			}
		}
	}
}
