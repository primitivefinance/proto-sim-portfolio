use ethers;

pub static WAD: f64 = 1_000_000_000_000_000_000.0;
pub static ARBITRAGEUR_ADDRESS_BASE: u64 = 2_u64;
pub static FEE_BPS: u16 = 10;
pub static VOLATILITY_BPS: u32 = 1_000;
pub static BASIS_POINT_DIVISOR: u16 = 10_000;

pub trait Endian {
    fn down_endian(&self) -> ethers::types::U256;
}

impl Endian for ethers::types::I256 {
    fn down_endian(&self) -> ethers::types::U256 {
        let mut buf = [0_u8; 32];
        self.to_little_endian(&mut buf);
        ethers::types::U256::from(&buf)
    }
}
