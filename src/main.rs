use anyhow::{anyhow, Result};
use clap::Parser;
use crossterm::style::Stylize;
use itertools::Itertools;
use rustyline::{
	completion::{Completer, Pair},
	highlight::Highlighter,
	hint::Hinter,
	validate::Validator,
	CompletionType, Context, EditMode, Helper, OutputStreamType,
};
use std::{
	fs,
	path::PathBuf,
	sync::{Arc, Mutex},
};

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
mod web;

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

#[derive(PartialEq, Eq, Debug, Clone, Copy, clap::ArgEnum)]
enum CalculationType {
	Dose,
	Target,
}

impl From<CalculationType> for concentration::DiluteCalcType {
	fn from(ct: CalculationType) -> Self {
		match ct {
			CalculationType::Dose => concentration::DiluteCalcType::ResultOfDose,
			CalculationType::Target => concentration::DiluteCalcType::TargetDose,
		}
	}
}

pub struct FertInputHelper {
	fert_hints: Vec<Pair>,
}
impl FertInputHelper {
	fn new(fertilizers_db: &FertilizersDb) -> Self {
		Self {
			fert_hints: fertilizers_db
				.known_fertilizers
				.iter()
				.map(|(fname, _)| Pair { display: fname.clone(), replacement: fname.clone() })
				.collect::<Vec<_>>(),
		}
	}
}

impl Helper for FertInputHelper {}
impl Hinter for FertInputHelper {
	type Hint = String;
}
impl Highlighter for FertInputHelper {}
impl Validator for FertInputHelper {}
impl Completer for FertInputHelper {
	type Candidate = Pair;
	fn complete(&self, line: &str, _pos: usize, _ctx: &Context) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
		let candidates = self
			.fert_hints
			.iter()
			.filter(|cn| cn.display.starts_with(line))
			.map(|p| Pair { display: p.display.clone(), replacement: p.replacement.clone() })
			.collect::<Vec<_>>();

		Ok((0, candidates))
	}
}

#[derive(Debug, Parser)]
pub(crate) struct Opts {
	/// Path to the elements toml database to use instead of the embedded one
	#[clap(long, parse(from_os_str))]
	elements: Option<PathBuf>,
	/// How a tank data is added
	#[clap(long, arg_enum, default_value = "volume")]
	tank_input: TankInputMode,
	/// Optional path for a toml file with tank definition
	#[clap(long, parse(from_os_str))]
	tank_toml: Option<PathBuf>,
	/// How a fertiliser is added
	#[clap(long, arg_enum, default_value = "dry")]
	dosing_method: DosingMethod,
	/// What type of fertilizer is checked
	#[clap(long, arg_enum, default_value = "any")]
	fertilizer: FertilizerType,
	/// Path to fertilizers database in toml format instead of the embedded database
	#[clap(long, parse(from_os_str))]
	database: Vec<PathBuf>,
	/// What type of calculation is desired
	#[clap(long, arg_enum, default_value = "dose")]
	calc: CalculationType,
	/// List the available fertilizers loaded from the database and exit
	#[clap(long, short = 'l')]
	list: bool,
	/// Use absolute volume without corrections
	#[clap(long, short = 'a')]
	absolute: bool,
	/// Work as a web server
	#[clap(long, short = 's')]
	serve: bool,
}

#[actix_web::main]
async fn main() -> Result<()> {
	let opts = Opts::parse();

	let known_elements = if let Some(elts_path) = opts.elements {
		elements::KnownElements::new_with_db(elts_path.as_path())
	} else {
		// Avoid hassle for generic users
		let known_elements_toml = include_str!("../elements.toml");
		elements::KnownElements::new_with_string(known_elements_toml)
	}?;

	let mut fertilizers_db: FertilizersDb = Default::default();

	let known_fertilizers_toml = include_str!("../fertilizers.toml");
	fertilizers_db.load_db(known_fertilizers_toml, &known_elements)?;

	for extra_db in opts.database.iter() {
		let data = fs::read_to_string(extra_db.as_path())?;
		fertilizers_db.load_db(data.as_str(), &known_elements)?;
	}

	if opts.list {
		for fert_name in fertilizers_db.known_fertilizers.keys().sorted() {
			println!("{}", fert_name);
		}

		return Ok(())
	}

	if opts.serve {
		return web::run_server(Arc::new(Mutex::new(fertilizers_db)))
			.await
			.map_err(|e| anyhow!("server error: {:?}", e))
	}

	let config = rustyline::Config::builder()
		.completion_type(CompletionType::List)
		.edit_mode(EditMode::Vi)
		.output_stream(OutputStreamType::Stdout)
		.build();
	let mut fert_editor = rustyline::Editor::with_config(config);
	fert_editor.set_helper(Some(FertInputHelper::new(&fertilizers_db)));

	let mut generic_editor = rustyline::Editor::<()>::with_config(config);

	let fertilizer: Box<dyn Fertilizer + Send> = match opts.fertilizer {
		FertilizerType::Any => {
			let input: String =
				fert_editor.readline("Input a fertilizer (e.g. `Miracle Gro`) or a compound (e.g. KNO3): ")?;

			let maybe_known_fertilizer = fertilizers_db.known_fertilizers.get(input.as_str());

			match maybe_known_fertilizer {
				Some(fertilizer_box) => {
					println!("Fertilizer: {}", fertilizer_box.name().bold());
					println!("Compounds by elements");
					let components = fertilizer_box.components_percentage(&known_elements);

					for displayed_elt in components {
						println!("{:?}", displayed_elt);
					}
					dyn_clone::clone(fertilizer_box)
				},
				None => {
					let compound = compound::Compound::new(input.as_str(), &known_elements)?;
					println!("Compound: {}", compound.name().bold());
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
			let compound = compound::Compound::new_from_stdin(&known_elements, &mut generic_editor)?;
			println!("Compound: {}", compound.name().bold());
			println!("Molar mass: {}", compound.molar_mass().to_string().bold());
			println!("Compounds by elements");
			let components = compound.components_percentage(&known_elements);

			for displayed_elt in components {
				println!("{:?}", displayed_elt);
			}
			Box::new(compound)
		},
		FertilizerType::Mix => {
			let mix = mix::MixedFertilizer::new_from_stdin(&known_elements, &mut fert_editor)?;
			println!("Mix: {}", mix.name().bold());
			println!("Compounds by elements");
			let components = mix.components_percentage(&known_elements);

			for displayed_elt in components {
				println!("{:?}", displayed_elt);
			}
			Box::new(mix)
		},
	};

	let tank = if let Some(tank_toml) = &opts.tank_toml {
		let data = fs::read_to_string(tank_toml.as_path())?;
		tank::Tank::new_from_toml(data.as_str())?
	} else if opts.tank_input == TankInputMode::Linear {
		tank::Tank::new_from_stdin_linear(opts.absolute, &mut generic_editor)?
	} else {
		tank::Tank::new_from_stdin_volume(opts.absolute, &mut generic_editor)?
	};

	println!("{:?}", &tank);

	let dosages =
		match opts.dosing_method {
			DosingMethod::Dry =>
				concentration::DryDosing::new_from_stdin(opts.calc.into(), &known_elements, &mut generic_editor)?
					.dilute(&*fertilizer, &known_elements, &tank)?,
			DosingMethod::Solution =>
				concentration::SolutionDosing::new_from_stdin(opts.calc.into(), &known_elements, &mut generic_editor)?
					.dilute(&*fertilizer, &known_elements, &tank)?,
		};

	if opts.calc == CalculationType::Target {
		println!("You need to add {:.3} grams of fertilizer to reach your target", dosages.compound_dose);
	}
	println!("Dose by elements");

	for dosage in dosages.elements_dose {
		println!("{:?}", &dosage);
	}

	Ok(())
}
