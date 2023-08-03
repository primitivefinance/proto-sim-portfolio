pub mod trading_function;

/// Available analyses.
pub enum Analysis {
    TradingFunction(TradingFunctionSubtype),
}

/// Specific analysis to conduct on Trading Function analysis class.
pub enum TradingFunctionSubtype {
    Error,
    Curve,
}

impl Default for TradingFunctionSubtype {
    fn default() -> Self {
        TradingFunctionSubtype::Error
    }
}
