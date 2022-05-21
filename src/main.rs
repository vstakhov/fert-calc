use anyhow::Result;
use clap::Parser;
use crossterm::style::Stylize;
use std::path::Path;

pub(crate) mod compound;
pub(crate) mod elements;

#[derive(Debug, Parser)]
pub(crate) struct Opts {
	/// Path to the elements json database to use instead of the embedded one
	#[clap(name = "elements", long)]
	elements: Option<String>,
}

fn main() -> Result<()> {
	let opts = Opts::parse();

	let known_elements = if let Some(elts_path) = opts.elements {
		elements::KnownElements::new_with_db(Path::new(elts_path.as_str()))
	} else {
		let known_elements_json = include_str!("../elements.json");
		elements::KnownElements::new_with_string(known_elements_json)
	}?;

	let compound = compound::Compound::from_stdin(&known_elements)?;
	println!("Compound: {}", compound.name.clone().bold());
	println!("Molar mass: {}", compound.molar_mass().to_string().bold());
	println!("Compounds by elements");

	for displayed_elt in compound.components_percentage(&known_elements) {
		println!("{:?}", displayed_elt);
	}
	Ok(())
}
