use bitflags::bitflags;
use dcc::FunctionGroupByte;
use loco_core::{
	drive::{Direction, Speed},
	functions::FunctionGroupNumber,
	mov, xor, Bits,
};
use loco_dcc as dcc;

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
	#[inline]
	fn bits(&self) -> u8 {
		self.bits
	}
}

#[derive(Debug, Clone)]
pub struct Accessory {
	address: u8,
	data: u8,
}

#[derive(Debug, Clone)]
pub enum SearchResult {
	Loco(u16),
	DoubleHeading(u16),
	ConsistBase(u16),
	Consist(u16),
	None,
}

#[derive(Debug, Clone)]
pub enum CentralError {
	ConsistError,
	ConsistOccupied,
	AlreadyInConsist,
	ConsistSpeedNotZero,
	LocoNotInConsist,
	NoConsistBase,
	DeleteNotPossible,
	StackOverflow,
}

#[derive(Debug, Clone)]
pub enum CentralMessage<S: Bits<u8>> {
	TrackPowerOn,
	TrackPowerOff,
	EmergencyStop,
	ProgrammingModeOn,
	FeedbackBroadcast([(u8, u8); 7]),
	ProgrammingShortCircuit,
	ProgrammingNoData,
	ProgrammingBusy,
	ProgrammingReady,
	ProgrammingDataPaged(u8, u8),
	ProgrammingDataDirect(u16, u8),
	Version(u8, u8),
	State(S),
	TransferError,
	StationBusy,
	UnknownCommand,
	AccessoryResponse(Accessory), // FIXME
	LocoInformation {
		is_free: bool,
		direction: Direction,
		speed: Speed,
		f0: FunctionGroupByte,
		f1: FunctionGroupByte,
	},
	FunctionState {
		f3: FunctionGroupByte,
		f4: FunctionGroupByte,
	},
	LocoConsistInformation {
		is_free: bool,
		direction: Direction,
		speed: Speed,
		f0: FunctionGroupByte,
		f1: FunctionGroupByte,
		consist_address: u8,
	},
	LocoConsistBaseInformation {
		is_free: bool,
		direction: Direction,
		speed: Speed,
	},
	LocoDoubleHeadingInformation {
		is_free: bool,
		direction: Direction,
		speed: Speed,
		f0: FunctionGroupByte,
		f1: FunctionGroupByte,
		other_address: u16,
	},
	LocoOccupied(u16),
	FunctionToggled0 {
		f0: FunctionGroupByte,
		f1: FunctionGroupByte,
	},
	FunctionToggled1 {
		f2: FunctionGroupByte,
		f3: FunctionGroupByte,
	},
	SearchResult(SearchResult),
	Error(CentralError),
	#[cfg(feature = "z21")]
	Z21LocoInformation {
		loco_address: u16,
		is_free: bool,
		direction: Direction,
		speed: Speed,
		f0: FunctionGroupByte,
		f1: FunctionGroupByte,
		f2: FunctionGroupByte,
		f3: FunctionGroupByte,
		double_heading: bool,
		smart_search: bool,
	},
}

#[derive(Debug)]
pub enum Error {
	ParseError,
}

impl<S: Bits<u8>> CentralMessage<S> {
	pub fn to_buf(&self, buf: &mut [u8]) -> usize {
		use CentralMessage::*;
		let add_xor = |buf: &mut [u8], len: usize| -> usize {
			let x = buf[0..len - 1].iter().fold(0, |acc, x| acc ^ x);
			buf[len - 1] = x;
			len
		};
		match self {
			TrackPowerOn => mov!(buf[0..3] <= &xor!([0x61, 0x01])),
			TrackPowerOff => mov!(buf[0..3] <= &xor!([0x61, 0x00])),
			EmergencyStop => mov!(buf[0..3] <= &xor!([0x81, 0x00])),
			Version(u, l) => mov!(buf[0..5] <= &xor!([0x63, 0x21, *u, *l])),
			State(state) => mov!(buf[0..4] <= &xor!([0x62, 0x22, state.bits()])),
			TransferError => mov!(buf[0..3] <= &xor!([0x61, 0x80])),
			StationBusy => mov!(buf[0..3] <= &xor!([0x61, 0x81])),
			UnknownCommand => mov!(buf[0..3] <= &xor!([0x61, 0x82])),
			#[cfg(feature = "z21")]
			Z21LocoInformation {
				loco_address,
				is_free,
				direction,
				speed,
				f0,
				f1,
				f2,
				f3,
				double_heading,
				smart_search,
			} => {
				buf[0] = 0xEF;
				mov!(buf[1..=2] <= &loco_address.to_le_bytes());
				let code = match speed {
					Speed::Steps14(_) => 0,
					Speed::Steps28(_) => 2,
					Speed::Steps128(_) => 4,
					_ => 4,
				};
				buf[3] = (*is_free as u8) << 3 | code;
				buf[4] = dcc::direction::to_advanced_byte(&direction) | dcc::speed::to_byte(&speed);
				buf[5] = (u8::from(*f0) & 0x3F)
					| ((*smart_search as u8) << 5)
					| ((*double_heading as u8) << 6);
				buf[6] = u8::from(*f1);
				buf[7] = u8::from(*f2);
				buf[8] = u8::from(*f3);
				add_xor(buf, 10)
			}
			_ => unimplemented!(),
		}
	}
}

#[derive(Debug, Clone)]
pub enum RefreshMode {
	F0ToF4 = 0x0,
	F0ToF8 = 0x1,
	F0ToF12 = 0x3,
	F0ToF20 = 0x7,
	F0ToF28 = 0xF,
}

#[derive(Debug, Clone)]
pub enum DeviceMessage {
	TrackPowerOn,
	TrackPowerOff,
	EmergencyStop,
	LocoEmergencyStop(u16),
	ProgrammingReadRegister(u8),
	ProgrammingReadDirect(u16),
	ProgrammingReadPaged(u8),
	ProgrammingGetResult,
	ProgrammingWriteRegister(u8, u8),
	ProgrammingWriteDirect(u16, u8),
	ProgrammingWritePaged(u8, u8),
	GetVersion,
	GetState,
	GetAccessory(Accessory),     // FIXME
	ControlAccessory(Accessory), // FIXME
	GetLocoInformation(u16),
	GetFunctionToggled0(u16),
	GetFunctionToggled1(u16),
	GetFunctionState(u16),
	LocoDrive(u16, Direction, Speed),
	SetFunctionGroup(FunctionGroupNumber, FunctionGroupByte),
	SetFunctionToggled(FunctionGroupNumber, FunctionGroupByte),
	SetRefreshMode(RefreshMode),
	AddDoubleHeading(u16, u16),
	RemoveDoubleHeading(u16),
	ProgrammingOnMainWrite {
		loco_address: u16,
		cv_address: u16,
		value: u8,
	},
	ProgrammingOnMainRead {
		loco_address: u16,
		cv_address: u16,
		value: u8,
	},
	ProgrammingOnMainWriteBit {
		loco_address: u16,
		cv_address: u16,
		position: u8,
		value: bool,
	},
	AddConsist {
		inverted: bool,
		loco_address: u16,
		base_address: u8,
	},
	RemoveConsist {
		loco_address: u16,
		base_address: u8,
	},
	SearchConsistMember {
		forward: bool,
		loco_address: u16,
		base_address: u8,
	},
	SearchConsistBase {
		forward: bool,
		base_address: u8,
	},
	SearchLocoInStack {
		forward: bool,
		loco_address: u16,
	},
	RemoveFromStack(u16),
}

impl DeviceMessage {
	pub fn from_bytes(bytes: &[u8]) -> Result<DeviceMessage, Error> {
		let check_xor = |len: usize, result: DeviceMessage| {
			let x = bytes[0..len - 1].iter().fold(0, |acc, x| acc ^ x);
			if x != bytes[len - 1] {
				Err(Error::ParseError)
			} else {
				Ok(result)
			}
		};
		use DeviceMessage::*;
		match bytes {
			[0x21, 0x81, 0xA0, ..] => Ok(TrackPowerOn),
			[0x21, 0x80, 0xA1, ..] => Ok(TrackPowerOff),
			[0x80, 0x80, ..] => Ok(EmergencyStop),
			[0x92, h, l, _, ..] => check_xor(4, LocoEmergencyStop(u16::from_le_bytes([*h, *l]))),
			[0x21, 0x21, 0x00, ..] => Ok(GetVersion),
			[0x21, 0x24, 0x05, ..] => Ok(GetState),
			[0xE4, 0x10, h, l, rv, _, ..] => check_xor(
				6,
				LocoDrive(
					u16::from_le_bytes([*h, *l]),
					dcc::direction::from_advanced_byte(*rv),
					dcc::speed::from_byte_14_steps(*rv),
				),
			),
			[0xE4, 0x12, h, l, rv, _, ..] => check_xor(
				6,
				LocoDrive(
					u16::from_le_bytes([*h, *l]),
					dcc::direction::from_advanced_byte(*rv),
					dcc::speed::from_byte_28_steps(*rv),
				),
			),
			[0xE4, 0x13, h, l, rv, _, ..] => check_xor(
				6,
				LocoDrive(
					u16::from_le_bytes([*h, *l]),
					dcc::direction::from_advanced_byte(*rv),
					dcc::speed::from_byte_128_steps(*rv),
				),
			),
			_ => {
				println!("X: {:#04X?}", bytes);
				Err(Error::ParseError)
			}
		}
	}
}
