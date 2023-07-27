use super::raw_data::*;
/// Traits and functions for converting RawData
/// into human readable spreadsheets!
use polars::prelude::*;

pub trait Spreadsheet {
    /// Converts the raw pool series data into a spreadsheet "data frame".
    fn to_spreadsheet(&self, key: u64) -> DataFrame;
}

impl Spreadsheet for RawData {
    fn to_spreadsheet(&self, pool_id: u64) -> DataFrame {
        // Empty spreadsheet...
        let mut df = df!(
            "reserves_x" => self.get_pool_x_per_lq_float(pool_id),
            "reserves_y" => self.get_pool_y_per_lq_float(pool_id),
            "reported_price" => self.get_reported_price_float(pool_id),
            "ref_price" => self.get_exchange_price_float(pool_id),
            "pvf" => self.get_portfolio_value_float(pool_id),
            "invariant" => self.get_invariant_float(pool_id),
            "arb_reserve_x" => self.get_arber_reserve_x_float(),
            "arb_reserve_y" => self.get_arber_reserve_y_float(),
            "arb_pvf" => self.get_arber_portfolio_value_float(pool_id),
        )
        .unwrap();

        df
    }
}
