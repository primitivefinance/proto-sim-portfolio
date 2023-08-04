/// Make plots for the raw sim data easily using the csv generated on sim run.
use polars::prelude::*;
use visualize::{design::*, plot::*};

/// Uses a Plot Display and DataFrame (i.e. csv) to make plots of the simulation data.
pub struct Plot {
    display: Display,
    data: DataFrame,
}

/// Implements utilites for plotting the csv data output from simulations.
#[allow(unused)]
impl Plot {
    /// constructor
    pub fn new(display: Display, data: DataFrame) -> Self {
        Self { display, data }
    }

    /// Loads a csv file from the given path.
    pub fn load_from_path(
        display: Display,
        path: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let data = CsvReader::from_path(path)?.finish()?;
        Ok(Self::new(display, data))
    }

    pub fn prices(&self) -> Vec<Series> {
        // gets the reported prices and ref prices
        let reported_price = self.data.column("reported_price").unwrap();
        let ref_price = self.data.column("ref_price").unwrap();

        vec![reported_price.clone(), ref_price.clone()]
    }

    pub fn pvfs(&self) -> Vec<Series> {
        let pvf = self.data.column("pvf").unwrap();
        let arb_pvf = self.data.column("arb_pvf").unwrap();

        vec![pvf.clone(), arb_pvf.clone()]
    }

    /// Gets the data container for every point on a line to be plotted.
    /// # Arguments
    /// * `title` - The title of the plot.
    /// * `series` - A vector of series pairs to plot. Should be in the form [(x, y), (x, y), ...]
    pub fn make_curves(&self, title: &str, series: Vec<(Series, Series)>) -> Vec<Curve> {
        let mut curves: Vec<Curve> = Vec::new();

        for (i, series) in series.iter().enumerate() {
            let color = match i {
                0 => Color::Purple,
                1 => Color::Blue,
                2 => Color::Green,
                _ => Color::Black,
            };

            // converts each series data into float64 vectors
            let x_coordinates = series.0.f64().expect("error converting x series to f64");
            let x_coordinates: Vec<f64> = x_coordinates
                .into_iter()
                .filter_map(|opt_f| opt_f)
                .collect();

            let y_coordinates = series.1.f64().expect("error converting y series to f64");
            let y_coordinates: Vec<f64> = y_coordinates
                .into_iter()
                .filter_map(|opt_f| opt_f)
                .collect();

            let curve = Curve {
                x_coordinates,
                y_coordinates,
                design: CurveDesign {
                    color,
                    color_slot: i.into(),
                    style: Style::Lines(LineEmphasis::Light),
                },
                name: Some(format!("{}", title)),
            };
            curves.push(curve);
        }

        curves
    }

    /// Plots each line of (x,y) coordinates.
    /// # Arguments
    /// * `directory` - The directory to save the plot to. It should exist.
    /// * `file` - The name of the file to save the plot to.
    /// * `title` - The title of the plot.
    /// * `curves` - Each Curve should have the same number of x and y coordinates. The x coordinates should map the range of the function.
    pub fn plot(&self, directory: &str, file: &str, title: &str, curves: Vec<Curve>) {
        let x_coordinates_flat = curves
            .iter()
            .flat_map(|curve| curve.x_coordinates.clone())
            .collect::<Vec<f64>>();

        let y_coordinates_flat = curves
            .iter()
            .flat_map(|curve| curve.y_coordinates.clone())
            .collect::<Vec<f64>>();

        if let Some(last_point) = x_coordinates_flat.last() {
            // Finds the minimum and maximum y across the entire y coordinates.
            let min_y = y_coordinates_flat
                .iter()
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap();
            let max_y = y_coordinates_flat
                .iter()
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap();

            let axes = Axes {
                x_label: String::from("X"),
                y_label: String::from("Y"), // todo: add better y label
                bounds: (
                    vec![x_coordinates_flat[0], *last_point],
                    vec![*min_y, *max_y],
                ),
            };

            transparent_plot(
                Some(curves),
                None,
                axes,
                title.to_string(),
                self.display.clone(),
                Some(format!(
                    "{}/{}.html",
                    directory.to_string(),
                    file.to_string()
                )),
            );
        } else {
            panic!("no x coordinates found");
        }
    }

    /// Makes a line plot for each given series of y coordinates.
    /// # Arguments
    /// * `y_coords_vec` - For each line, a series a y coordinates. Each element in the root vector should have the same length.
    pub fn stacked_line_plot(&self, y_coords_vec: Vec<Vec<f64>>, title: &str) {
        let length = y_coords_vec[0].len();
        // Equally spaced x coordinates.
        let x_coordinates =
            itertools_num::linspace(0.0, length as f64, length).collect::<Vec<f64>>();

        let names = vec!["spot".to_string(), "ref".to_string()];
        // get a curve for each y coordinate vector
        let curves = y_coords_vec
            .iter()
            .enumerate()
            .map(|(i, y_coordinates)| Curve {
                x_coordinates: x_coordinates.clone(),
                y_coordinates: y_coordinates.clone(),
                design: CurveDesign {
                    color: match i {
                        0 => Color::Purple,
                        1 => Color::Blue,
                        2 => Color::Green,
                        _ => Color::Black,
                    },
                    color_slot: 1,
                    style: Style::Lines(LineEmphasis::Light),
                },
                name: Some(format!("{}", names[i])),
            })
            .collect::<Vec<Curve>>();

        self.plot("./out_data", title, title, curves);
    }

    /// Plots the reported price and reference prices on two lines on the same graph.
    pub fn stacked_price_plot(&self) {
        // get the reported and ref prices into a vector
        let prices = self.prices();

        // make a stacked line plot for each of the prices
        self.stacked_line_plot(
            vec![
                prices[0]
                    .f64()
                    .expect("error converting reported price to f64")
                    .into_iter()
                    .filter_map(|opt_f| opt_f)
                    .into_iter()
                    .collect::<Vec<f64>>(),
                prices[1]
                    .f64()
                    .expect("error converting ref price to f64")
                    .into_iter()
                    .filter_map(|opt_f| opt_f)
                    .into_iter()
                    .collect::<Vec<f64>>(),
            ],
            "prices",
        );
    }

    /// Plots the x and y reserves of a given pool data series on two lines on the same graph.
    pub fn stacked_reserves_plot(&self) {
        todo!()
    }

    /// Plots the LP potfolio value and the arbitrageur's portfolio value on two lines on the same graph.
    pub fn stacked_portfolio_value_plot(&self) {
        // get the LP pvf and arber pvf
        let pvfs = self.pvfs();

        // make a stacked line plot for each of the pvfs
        self.stacked_line_plot(
            vec![
                pvfs[0]
                    .f64()
                    .expect("error converting reported price to f64")
                    .into_iter()
                    .filter_map(|opt_f| opt_f)
                    .into_iter()
                    .collect::<Vec<f64>>(),
                pvfs[1]
                    .f64()
                    .expect("error converting ref price to f64")
                    .into_iter()
                    .filter_map(|opt_f| opt_f)
                    .into_iter()
                    .collect::<Vec<f64>>(),
            ],
            "portfolios",
        );
    }
    pub fn lp_pvf_plot(&self) {
        // get the LP pvf and arber pvf
        let pvfs = self.pvfs();

        // make a stacked line plot for each of the pvfs
        self.stacked_line_plot(
            vec![pvfs[0]
                .f64()
                .expect("error converting reported price to f64")
                .into_iter()
                .filter_map(|opt_f| opt_f)
                .into_iter()
                .collect::<Vec<f64>>()],
            "lp_pvf",
        );
    }

    pub fn arbitrageur_pvf_plot(&self) {
        // get the LP pvf and arber pvf
        let pvfs = self.pvfs();

        // make a stacked line plot for each of the pvfs
        self.stacked_line_plot(
            vec![pvfs[1]
                .f64()
                .expect("error converting ref price to f64")
                .into_iter()
                .filter_map(|opt_f| opt_f)
                .into_iter()
                .collect::<Vec<f64>>()],
            "arbitrageur_pvf",
        );
    }
}

/// Gets the minimum and maximum values from a list of coordinates.
pub fn get_coordinate_bounds(coords_list: Vec<Vec<f64>>) -> (f64, f64) {
    let flat = coords_list
        .iter()
        .flat_map(|coord| coord.clone())
        .collect::<Vec<f64>>();

    let min = flat
        .iter()
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();

    let max = flat
        .iter()
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap();

    (*min, *max)
}
