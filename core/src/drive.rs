#[derive(Clone, Debug, PartialEq, Copy)]
pub enum Direction {
    Forward,
    Backward,
}

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum Speed {
    Stop,
    EmergencyStop,
    Steps14(u8),
    Steps28(u8),
    Steps128(u8),
}
