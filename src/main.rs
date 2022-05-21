use anyhow::Result;
use clap::Parser;
use crossterm::style::Stylize;
use std::{fs, path::PathBuf};

use crate::{concentration::DiluteMethod, traits::Fertilizer};

mod compound;
mod concentration;
mod elements;
mod tank;
mod traits;
#[macro_use]
mod test_utils;

#[derive(PartialEq, Eq, Debug, Clone, Copy, clap::ArgEnum)]
enum TankInputMode {
	Linear,
	Volume,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy, clap::ArgEnum)]
enum DosingMethod {
	Dry,
	Solution,
}

#[derive(Debug, Parser)]
pub(crate) struct Opts {
	/// Path to the elements json database to use instead of the embedded one
	#[clap(long, parse(from_os_str))]
	elements: Option<PathBuf>,
	/// How a tank data is added
	#[clap(long, arg_enum, default_value = "volume")]
	tank_input: TankInputMode,
	/// Optional path for a json file with tank definition
	#[clap(long, parse(from_os_str))]
	tank_json: Option<PathBuf>,
	/// How a fertiliser is added
	#[clap(long, arg_enum, default_value = "dry")]
	dosing_method: DosingMethod,
}

fn main() -> Result<()> {
	let opts = Opts::parse();

	let known_elements = if let Some(elts_path) = opts.elements {
		elements::KnownElements::new_with_db(elts_path.as_path())
	} else {
		// Avoid hassle for generic users
		let known_elements_json = include_str!("../elements.json");
		elements::KnownElements::new_with_string(known_elements_json)
	}?;

	let compound = compound::Compound::from_stdin(&known_elements)?;
	println!("Compound: {}", compound.name().clone().bold());
	println!("Molar mass: {}", compound.molar_mass().to_string().bold());
	println!("Compounds by elements");
	let mut components = compound.components_percentage(&known_elements);
	components.sort();

	for displayed_elt in components {
		println!("{:?}", displayed_elt);
	}

	let tank = if let Some(tank_json) = &opts.tank_json {
		let data = fs::read_to_string(tank_json.as_path())?;
		tank::Tank::new_from_json(data.as_str())?
	} else {
		if opts.tank_input == TankInputMode::Linear {
			tank::Tank::new_from_stdin_linear()?
		} else {
			tank::Tank::new_from_stdin_volume()?
		}
	};

	println!("{:?}", &tank);

	let fertilizer: Box<dyn Fertilizer> = Box::new(compound);
	let dosing: Box<dyn DiluteMethod> = match opts.dosing_method {
		DosingMethod::Dry => Box::new(concentration::DryDosing::new_from_stdin()?),
		DosingMethod::Solution => Box::new(concentration::SolutionDosing::new_from_stdin()?),
	};
	let mut dosages = dosing.dilute(&fertilizer, &known_elements, &tank);
	dosages.sort();

	println!("Dose by elements");

	for dosage in dosages {
		println!("{:?}", &dosage);
	}

	Ok(())
}
