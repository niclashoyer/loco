use bitflags::bitflags;
use embedded_nal::{UdpClient, UdpServer};
use loco_core::Bits;
use loco_xpressnet as xnet;

bitflags! {
	pub struct CentralStateEx: u8 {
		const HIGH_TEMPERATURE = 0b0000_0001;
		const POWER_LOST = 0b0000_0010;
		const SHORT_CIRCUIT_EXTERNAL = 0b0000_0100;
		const SHORT_CIRCUIT_INTERNAL = 0b0000_1000;
	}
}

bitflags! {
	pub struct CentralState: u8 {
		const EMERGENCY_OFF = 0b0000_0001;
		const EMERGENCY_STOP = 0b0000_0010;
		const SHORT_CIRCUIT = 0b0000_0100;
		const PROGRAMMING_MODE = 0b0010_0000;
	}
}

impl Bits<u8> for CentralState {
	fn bits(&self) -> u8 {
		self.bits
	}
}

bitflags! {
	pub struct BroadcastFlags: u32 {
		const DRIVING_SWITCHING = 0x00000001;
		const RBUS = 0x00000002;
		const RAILCOM = 0x00000004;
		const SYSTEM_STATUS = 0x00000100;
		const DRIVING_SWITCHING_ALL = 0x00010000;
		const LOCONET = 0x01000000;
		const LOCONET_LOCO = 0x02000000;
		const LOCONET_SWITCH = 0x04000000;
		const LOCONET_OCCUPY = 0x08000000;
		const RAILCOM_ALL = 0x00040000;
		const CAN_OCCUPY = 0x00080000;
	}
}

#[derive(Debug)]
pub enum HardwareType {
	Z21Old,
	Z21New,
	SmartRail,
	Z21Small,
	Z21Start,
	Custom(u32),
}

#[derive(Debug)]
pub struct FirmwareVersion {
	major: u8,
	minor: u8,
}

#[derive(Debug)]
pub enum CentralMessage {
	HardwareInfo(HardwareType, FirmwareVersion),
	SerialNumber(u32),
	SystemState {
		main_current: i16,
		prog_current: i16,
		filtered_main_current: i16,
		temperature: i16,
		supply_voltage: u16,
		vcc_voltage: u16,
		central_state: CentralState,
		central_state_ex: CentralStateEx,
	},
	BroadcastFlags(BroadcastFlags),
	XpressNet(xnet::CentralMessage<CentralState>),
}

macro_rules! mov {
	( $b:ident[$p:expr] <= $($x:tt)* ) => {{
		$b[$p].copy_from_slice($($x)*);
		$b[$p].len()
	}};
}

impl CentralMessage {
	pub fn to_buf(&self, buf: &mut [u8]) -> usize {
		use CentralMessage::*;
		match self {
			SystemState {
				main_current,
				prog_current,
				filtered_main_current,
				temperature,
				supply_voltage,
				vcc_voltage,
				central_state,
				central_state_ex,
			} => {
				mov!(buf[0..=3] <= &[0x14, 0x00, 0x84, 0x00]);
				mov!(buf[4..=5] <= &main_current.to_le_bytes());
				mov!(buf[6..=7] <= &prog_current.to_le_bytes());
				mov!(buf[8..=9] <= &filtered_main_current.to_le_bytes());
				mov!(buf[10..=11] <= &temperature.to_le_bytes());
				mov!(buf[12..=13] <= &supply_voltage.to_le_bytes());
				mov!(buf[14..=15] <= &vcc_voltage.to_le_bytes());
				mov!(buf[16..=19] <= &[central_state.bits, central_state_ex.bits, 0x00, 0x00]);
				20
			}
			SerialNumber(num) => {
				mov!(buf[0..=1] <= &[0x10, 0x11]);
				mov!(buf[2..=5] <= &num.to_le_bytes());
				6
			}
			XpressNet(xmsg) => {
				mov!(buf[2..=3] <= &[0x40, 0x00]);
				let xnum = xmsg.to_buf(&mut buf[4..]);
				let size = 4 + xnum;
				mov!(buf[0..=1] <= &(size as u16).to_le_bytes());
				size
			}
			_ => unimplemented!(),
		}
	}
}

#[derive(Debug)]
pub enum ClientMessage {
	GetHardwareInfo,
	GetSerialNumber,
	GetSystemState,
	GetBroadcastFlags,
	SetBroadcastFlags(BroadcastFlags),
	XpressNet(xnet::DeviceMessage),
}

impl ClientMessage {
	pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
		use ClientMessage::*;
		if bytes.len() < 4 {
			return Err(Error::ParseCommand);
		}
		let len = u16::from_le_bytes([bytes[0], bytes[1]]) as usize;
		let header = &bytes[2..4];
		match header {
			[0x40, 0x00] => Ok(XpressNet(xnet::DeviceMessage::from_bytes(&bytes[4..len])?)),
			[0x85, 0x00] => Ok(GetSystemState),
			[0x10, 0x00] => Ok(GetSerialNumber),
			[0x1A, 0x00] => Ok(GetHardwareInfo),
			_ => {
				println!("Unknown: {:#04X?}", bytes);
				Err(Error::ParseCommand)
			}
		}
	}
}

#[derive(Debug)]
pub enum Error {
	Receive,
	Send,
	Bind,
	ParseCommand,
	XpressNet(xnet::Error),
}

impl From<xnet::Error> for Error {
	fn from(e: xnet::Error) -> Error {
		Error::XpressNet(e)
	}
}

const BUF_SIZE: usize = 64;

struct Server<S>
where
	S: Sized,
{
	socket: S,
	recv_buf: [u8; BUF_SIZE],
	send_buf: [u8; BUF_SIZE],
}

type ClientAddress = embedded_nal::SocketAddr;

impl<S> Server<S>
where
	S: Sized,
{
	pub fn new(socket: S) -> Self {
		Self {
			socket,
			recv_buf: [0; BUF_SIZE],
			send_buf: [0; BUF_SIZE],
		}
	}

	pub fn send<U, E>(
		&mut self,
		server: &U,
		client: ClientAddress,
		message: &CentralMessage,
	) -> nb::Result<(), Error>
	where
		U: UdpServer<Error = E, UdpSocket = S>,
		E: core::fmt::Debug,
	{
		let len = message.to_buf(&mut self.send_buf);
		//println!("sending: ({:?},{:?})", client, message);
		//println!("{:#04X?}", &self.send_buf[0..len]);
		server
			.send_to(&mut self.socket, client, &self.send_buf[0..len])
			.map_err(|e| e.map(|_| Error::Send))
	}

	pub fn receive<U, E>(&mut self, server: &U) -> nb::Result<(ClientAddress, ClientMessage), Error>
	where
		U: UdpServer<Error = E, UdpSocket = S>,
		E: core::fmt::Debug,
	{
		let (num, addr) = server
			.receive(&mut self.socket, &mut self.recv_buf)
			.map_err(|e| e.map(|_| Error::Receive))?;
		let msg = ClientMessage::from_bytes(&self.recv_buf[0..num])?;
		//println!("received: ({:?},{:?})", addr, msg);
		//println!("{:#04X?}", &self.recv_buf[0..num]);
		Ok((addr, msg))
	}
}

fn main() {
	use nb::block;
	use std_embedded_nal::STACK;

	const PORT: u16 = 21105;

	let mut sock = STACK.socket().unwrap();
	STACK.bind(&mut sock, PORT).unwrap();

	println!("listening on port {}", PORT);

	let mut server = Server::new(sock);
	loop {
		let recv = block!(server.receive(&STACK));
		if let Ok((addr, msg)) = recv {
			use CentralMessage::*;
			use ClientMessage::*;
			match msg {
				GetSystemState => {
					let state = SystemState {
						main_current: 10,
						prog_current: 20,
						filtered_main_current: 30,
						temperature: 40,
						supply_voltage: 2000,
						vcc_voltage: 2000,
						central_state: CentralState::SHORT_CIRCUIT,
						central_state_ex: CentralStateEx::empty(),
					};
					block!(server.send(&STACK, addr, &state)).unwrap();
				}
				GetSerialNumber => {
					let sn = SerialNumber(58625);
					block!(server.send(&STACK, addr, &sn)).unwrap();
				}
				GetHardwareInfo => {
					let msg = HardwareInfo(
						HardwareType::Z21New,
						FirmwareVersion {
							major: 33,
							minor: 1,
						},
					);
					block!(server.send(&STACK, addr, &msg)).unwrap();
				}
				ClientMessage::XpressNet(xnet::DeviceMessage::GetVersion) => {
					let msg = CentralMessage::XpressNet(xnet::CentralMessage::Version(30, 0x12));
					block!(server.send(&STACK, addr, &msg)).unwrap();
				}
				ClientMessage::XpressNet(xnet::DeviceMessage::GetState) => {
					let state = CentralState::empty();
					let msg = CentralMessage::XpressNet(xnet::CentralMessage::State(state));
					block!(server.send(&STACK, addr, &msg)).unwrap();
					let msg = CentralMessage::XpressNet(xnet::CentralMessage::TrackPowerOn);
					block!(server.send(&STACK, addr, &msg)).unwrap();
				}
				_ => {}
			}
		}
	}
}
