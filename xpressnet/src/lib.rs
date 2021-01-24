use bitflags::bitflags;
use loco_core::{
	drive::{Direction, Speed},
	functions::FunctionGroupNumber,
	mov, xor, Bits,
};
use loco_dcc::FunctionGroupByte;

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
pub struct Accessory {
	address: u8,
	data: u8,
}

#[derive(Debug)]
pub enum SearchResult {
	Loco(u16),
	DoubleHeading(u16),
	ConsistBase(u16),
	Consist(u16),
	None,
}

#[derive(Debug)]
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

#[derive(Debug)]
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
}

#[derive(Debug)]
pub enum Error {
	ParseError,
}

impl<S: Bits<u8>> CentralMessage<S> {
	pub fn to_buf(&self, buf: &mut [u8]) -> usize {
		use CentralMessage::*;
		match self {
			TrackPowerOn => mov!(buf[0..3] <= &xor!([0x61, 0x01])),
			TrackPowerOff => mov!(buf[0..3] <= &xor!([0x61, 0x00])),
			EmergencyStop => mov!(buf[0..3] <= &xor!([0x81, 0x00])),
			Version(u, l) => mov!(buf[0..5] <= &xor!([0x63, 0x21, *u, *l])),
			State(state) => mov!(buf[0..4] <= &xor!([0x62, 0x22, state.bits()])),
			TransferError => mov!(buf[0..3] <= &xor!([0x61, 0x80])),
			StationBusy => mov!(buf[0..3] <= &xor!([0x61, 0x81])),
			UnknownCommand => mov!(buf[0..3] <= &xor!([0x61, 0x82])),
			_ => unimplemented!(),
		}
	}
}

#[derive(Debug)]
pub enum RefreshMode {
	F0ToF4 = 0x0,
	F0ToF8 = 0x1,
	F0ToF12 = 0x3,
	F0ToF20 = 0x7,
	F0ToF28 = 0xF,
}

#[derive(Debug)]
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
		use DeviceMessage::*;
		match bytes {
			[0x21, 0x21, 0x00, ..] => Ok(GetVersion),
			[0x21, 0x24, 0x05, ..] => Ok(GetState),
			_ => {
				println!("X: {:#04X?}", bytes);
				Err(Error::ParseError)
			}
		}
	}
}
