use bitflags::bitflags;
use embedded_nal::{UdpClient, UdpServer};
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

impl xnet::Bits<u8> for CentralState {
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
enum HardwareType {
	Z21Old,
	Z21New,
	SmartRail,
	Z21Small,
	Z21Start,
	Custom(u32),
}

#[derive(Debug)]
struct FirmwareVersion {
	major: u8,
	minor: u8,
}

#[derive(Debug)]
enum CentralMessage {
	HardwareInfo(HardwareType, FirmwareVersion),
	SystemStateChanged {
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

#[derive(Debug)]
enum ClientMessage {
	GetHardwareInfo,
	GetSystemState,
	GetBroadcastFlags,
	SetBroadcastFlags(BroadcastFlags),
}

impl ClientMessage {
	pub fn from_bytes(bytes: &[u8]) -> Result<Self, Z21Error> {
		if bytes.len() < 2 {
			return Err(Z21Error::ParseCommand);
		}
		let header = (bytes[0], bytes[1]);
		println!("{:#04X?}", bytes);
		match header {
			_ => Err(Z21Error::ParseCommand),
		}
	}
}

#[derive(Debug)]
enum Z21Error {
	Receive,
	Send,
	Bind,
	ParseCommand,
}

const BUF_SIZE: usize = 200;

struct Z21Server<S>
where
	S: Sized,
{
	socket: S,
	rec_buf: [u8; BUF_SIZE],
	send_buf: [u8; BUF_SIZE],
}

impl<S> Z21Server<S>
where
	S: Sized,
{
	pub fn new(socket: S) -> Self {
		Z21Server {
			socket,
			rec_buf: [0; BUF_SIZE],
			send_buf: [0; BUF_SIZE],
		}
	}

	pub fn send<U, E>(&mut self, server: &U) -> nb::Result<(), Z21Error>
	where
		U: UdpServer<Error = E, UdpSocket = S>,
		E: core::fmt::Debug,
	{
		Err(nb::Error::WouldBlock)
	}

	pub fn receive<U, E>(&mut self, server: &U) -> nb::Result<ClientMessage, Z21Error>
	where
		U: UdpServer<Error = E, UdpSocket = S>,
		E: core::fmt::Debug,
	{
		let (num, addr) = server
			.receive(&mut self.socket, &mut self.rec_buf)
			.map_err(|e| e.map(|_| Z21Error::Receive))?;
		let msg = ClientMessage::from_bytes(&self.rec_buf[0..num])?;
		Ok(msg)
	}
}

fn main() {
	use nb::block;
	use std_embedded_nal::STACK;

	const PORT: u16 = 21105;

	let mut sock = STACK.socket().unwrap();
	STACK.bind(&mut sock, PORT).unwrap();

	println!("listening on port {}", PORT);

	let mut server = Z21Server::new(sock);
	loop {
		let msg = block!(server.receive(&STACK));
		println!("received: {:?}", msg);
	}
}
