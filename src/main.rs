use anyhow::Result;
use clap::Parser;
use crossterm::style::Stylize;
use dialoguer::Input;
use std::{fs, path::PathBuf};

use crate::{concentration::DiluteMethod, fertilizers_db::FertilizersDb, traits::Fertilizer};

mod compound;
mod concentration;
mod elements;
mod fertilizers_db;
mod mix;
mod tank;
mod traits;

#[cfg(test)]
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

#[derive(PartialEq, Eq, Debug, Clone, Copy, clap::ArgEnum)]
enum FertilizerType {
	Any,
	Compound,
	Mix,
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
	/// What type of fertilizer is checked
	#[clap(long, arg_enum, default_value = "any")]
	fertilizer: FertilizerType,
	/// What type of fertilizer is checked
	#[clap(long, parse(from_os_str))]
	fertilizers_db: Option<PathBuf>,
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

	let mut fertilizers_db: FertilizersDb = Default::default();

	if let Some(fertilizers_db_path) = opts.fertilizers_db {
		let data = fs::read_to_string(fertilizers_db_path.as_path())?;
		fertilizers_db.load_db(data.as_str(), &known_elements)?;
	} else {
		let known_fertilizers_json = include_str!("../fertilizers.json");
		fertilizers_db.load_db(known_fertilizers_json, &known_elements)?;
	}

	let fertilizer: Box<dyn Fertilizer> = match opts.fertilizer {
		FertilizerType::Any => {
			let input: String = Input::new()
				.with_prompt("Input a fertilizer (e.g. `Miracle Gro`) or a compound (e.g. KNO3)")
				.interact_text()?;

			let maybe_known_fertilizer = fertilizers_db.known_fertilizers.get(input.as_str());

			match maybe_known_fertilizer {
				Some(fertilizer_box) => {
					println!("Fertilizer: {}", fertilizer_box.name().clone().bold());
					println!("Compounds by elements");
					let components = fertilizer_box.components_percentage(&known_elements);

					for displayed_elt in components {
						println!("{:?}", displayed_elt);
					}
					dyn_clone::clone(fertilizer_box)
				},
				None => {
					let compound = compound::Compound::new(input.as_str(),&known_elements)?;
					println!("Compound: {}", compound.name().clone().bold());
					println!("Molar mass: {}", compound.molar_mass().to_string().bold());
					println!("Compounds by elements");
					let components = compound.components_percentage(&known_elements);

					for displayed_elt in components {
						println!("{:?}", displayed_elt);
					}
					Box::new(compound)
				},
			}
		},
		FertilizerType::Compound => {
			let compound = compound::Compound::new_from_stdin(&known_elements)?;
			println!("Compound: {}", compound.name().clone().bold());
			println!("Molar mass: {}", compound.molar_mass().to_string().bold());
			println!("Compounds by elements");
			let components = compound.components_percentage(&known_elements);

			for displayed_elt in components {
				println!("{:?}", displayed_elt);
			}
			Box::new(compound)
		},
		FertilizerType::Mix => {
			let mix = mix::MixedFertilizer::new_from_stdin(&known_elements)?;
			println!("Mix: {}", mix.name().clone().bold());
			println!("Compounds by elements");
			let components = mix.components_percentage(&known_elements);

			for displayed_elt in components {
				println!("{:?}", displayed_elt);
			}
			Box::new(mix)
		},
	};

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

	let dosages = match opts.dosing_method {
		DosingMethod::Dry => concentration::DryDosing::new_from_stdin()?.dilute(&fertilizer, &known_elements, &tank),
		DosingMethod::Solution =>
			concentration::SolutionDosing::new_from_stdin()?.dilute(&fertilizer, &known_elements, &tank),
	};

	println!("Dose by elements");

	for dosage in dosages {
		println!("{:?}", &dosage);
	}

	Ok(())
}
