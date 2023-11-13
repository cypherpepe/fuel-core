use crate::{
    estimate_model,
    Model,
};
use serde::{
    Deserialize,
    Serialize,
};
use std::collections::{
    HashMap,
    VecDeque,
};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Costs(pub HashMap<String, Cost>);

impl Costs {
    pub fn with_capacity(size: usize) -> Self {
        Self(HashMap::with_capacity(size))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DependentCost {
    LightOperation { base: u64, units_per_gas: u64 },
    HeavyOperation { base: u64, gas_per_unit: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Cost {
    Relative(u64),
    Dependent(DependentCost),
}

fn slope(a: (f64, f64), b: (f64, f64)) -> f64 {
    let rise = b.1 - a.1;
    let run = b.0 - a.0;
    rise / run
}

pub fn dependent_cost(
    name: &String,
    points: Vec<(f64, f64)>,
) -> anyhow::Result<DependentCost> {
    let model = estimate_model(&points)?;
    let cost = match model {
        Model::Zero => {
            // Zero
            let warning = format!("Warning: Evaluating the regression on the dataset for {name} produced the zero function. This implies the cost behavior is independent of input and is not supported in a dependent context.", name = name);
            eprintln!("{}", warning);
            DependentCost::HeavyOperation {
                base: 0,
                gas_per_unit: 0,
            }
        }
        Model::Constant(coefficients) => {
            // Constant
            let warning = format!("Warning: Evaluating the regression on the dataset for {name} produced a constant function. This implies the cost behavior is independent of input and is not supported in a dependent context.", name = name);
            eprintln!("{}", warning);
            let base = coefficients.y;
            let base = base.max(0.0);
            let base = base.round() as u64;
            DependentCost::HeavyOperation {
                base,
                gas_per_unit: 0,
            }
        }
        Model::Linear(coefficients) => {
            match coefficients.slope {
                slope if slope > 0.0 && slope < 1.0 => {
                    // Slope is between (0.0, 1.0)
                    // Light operation
                    let base = coefficients.intercept;
                    let base = base.max(0.0);
                    let base = base.round() as u64;
                    let inverse_slope = 1.0 / slope;
                    let units_per_gas = inverse_slope.round() as u64;
                    DependentCost::LightOperation {
                        base,
                        units_per_gas,
                    }
                }
                slope if slope >= 1.0 => {
                    // Slope is greater than 1.0
                    // Heavy operation
                    let base = coefficients.intercept;
                    let base = base.max(0.0);
                    let base = base.round() as u64;
                    let gas_per_unit = slope.round() as u64;
                    DependentCost::HeavyOperation { base, gas_per_unit }
                }
                _ => {
                    // Slope is negative
                    let warning = format!("Warning: Evaluating the regression on the dataset for {name} produced a negative slope. This implies a negative cost behavior and is not supported in a dependent context.", name = name);
                    eprintln!("{}", warning);
                    let base = coefficients.intercept;
                    let base = base.round() as u64;
                    DependentCost::HeavyOperation {
                        base,
                        gas_per_unit: 0,
                    }
                }
            }
        }
        Model::Quadratic(_coefficients) => {
            // Quadratic
            let warning = format!("Warning: Evaluating the regression on the dataset for {name} produced a quadratic function. Quadratic behavior is not supported in a dependent context.", name = name);
            eprintln!("{}", warning);
            DependentCost::HeavyOperation {
                base: 0,
                gas_per_unit: 0,
            }
        }
        Model::Other => {
            // Other
            // This includes exponential and logarithmic functions
            let warning = format!("Warning: Evaluating the regression on the dataset for {name} produced a function that is not supported in a dependent context.", name = name);
            eprintln!("{}", warning);
            DependentCost::HeavyOperation {
                base: 0,
                gas_per_unit: 0,
            }
        }
    };
    Ok(cost)
}
