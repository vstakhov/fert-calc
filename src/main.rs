use anyhow::Result;
use crossterm::style::Stylize;
use dialoguer::Input;
use std::path::Path;

pub(crate) mod compound;
pub(crate) mod elements;

fn main() -> Result<()> {
	let known_elements = elements::KnownElements::new_with_db(Path::new("./elements.json"))?;
	let input_compound: String = Input::new().with_prompt("Input compound (e.g. KNO3)").interact_text()?;
	let compound = compound::Compound::new(input_compound.as_str(), &known_elements)?;
	println!("Compound: {}", compound.name.clone().bold());
	println!("Molar mass: {}", compound.molar_mass().to_string().bold());
	println!("Compounds by elements");

	for displayed_elt in compound.components_percentage(&known_elements) {
		println!("{:?}", displayed_elt);
	}
	Ok(())
}
