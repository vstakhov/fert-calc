use crate::{elements::KnownElements, tank::Tank, Fertilizer};
use anyhow::Result;
use crossterm::style::Stylize;
use dialoguer::Input;
use itertools::Itertools;
use serde::Deserialize;
use std::{
	cmp::Ordering,
	fmt::{Debug, Formatter},
};

/// Element name and it's concentration
pub struct ElementConcentration {
	pub element: String,
	pub concentration: f64,
}
/// Concentration of the all elements in a fertilizer with all elements' aliases
pub struct ElementsConcentrationsWithAliases {
	pub element: String,
	pub concentration: f64,
	pub aliases: Vec<ElementConcentration>,
}

impl Debug for ElementsConcentrationsWithAliases {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "Element: {} = {:.2}%", self.element.clone().bold(), self.concentration * 100.0)?;

		for alias in self.aliases.iter() {
			write!(f, " as {}: {:.2}%", alias.element.clone().bold(), alias.concentration * 100.0)?;
		}

		Ok(())
	}
}

/// Element name and it's dose
pub struct ElementDose {
	pub element: String,
	pub dose: f64,
}
/// Dosing of the all elements in a fertilizer with all elements' aliases
pub struct ElementsDosesWithAliases {
	pub element: String,
	pub dose: f64,
	pub aliases: Vec<ElementDose>,
}

// Helpers to output and sort structures
impl Debug for ElementsDosesWithAliases {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "Element: {} = {:.2} mg/l", self.element.clone().bold(), self.dose)?;

		for alias in self.aliases.iter() {
			write!(f, " as {}: {:.2} mg/l", alias.element.clone().bold(), alias.dose)?;
		}

		Ok(())
	}
}

impl PartialOrd for ElementsConcentrationsWithAliases {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.element.cmp(&&other.element))
	}
}

impl Ord for ElementsConcentrationsWithAliases {
	fn cmp(&self, other: &Self) -> Ordering {
		self.element.cmp(&&other.element)
	}
}

impl PartialEq for ElementsConcentrationsWithAliases {
	fn eq(&self, other: &Self) -> bool {
		self.element == other.element
	}
}

impl Eq for ElementsConcentrationsWithAliases {}

impl PartialOrd for ElementsDosesWithAliases {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.element.cmp(&&other.element))
	}
}

impl Ord for ElementsDosesWithAliases {
	fn cmp(&self, other: &Self) -> Ordering {
		self.element.cmp(&&other.element)
	}
}

impl PartialEq for ElementsDosesWithAliases {
	fn eq(&self, other: &Self) -> bool {
		self.element == other.element
	}
}

impl Eq for ElementsDosesWithAliases {}

/// Represents a concentration after adding some fertilizer to the specific tank
pub trait DiluteMethod {
	/// Load dilute method from stdin
	fn new_from_stdin() -> Result<Self>
	where
		Self: Sized;
	/// Deserialize dilute method from JSON
	fn new_from_json(json: &str) -> Result<Self>
	where
		Self: Sized;
	/// Dilute fertilizer in a specific tank using known dilute method
	fn dilute(
		&self,
		fertilizer: &Box<dyn Fertilizer>,
		known_elements: &KnownElements,
		tank: &Tank,
	) -> Vec<ElementsDosesWithAliases>;
}

/// A concrete implementation of the dosing with the value in grams
#[derive(Debug, Deserialize, Clone, Copy)]
pub struct DryDosing(f64);

impl DiluteMethod for DryDosing {
	fn new_from_stdin() -> Result<Self> {
		let input: String = Input::new().with_prompt("Dose size in grams (e.g. 2.5): ").interact_text()?;
		let dose = input.parse::<f64>()?;
		Ok(Self(dose))
	}

	fn new_from_json(json: &str) -> Result<Self> {
		let res: Self = serde_json::from_str(json)?;
		Ok(res)
	}

	fn dilute(
		&self,
		fertilizer: &Box<dyn Fertilizer>,
		known_elements: &KnownElements,
		tank: &Tank,
	) -> Vec<ElementsDosesWithAliases> {
		// For dry dosing we simply dilute all components by a tank's effective volume
		let mult = self.0 * 1000.0 / tank.effective_volume() as f64;
		let concentrations = fertilizer.components_percentage(known_elements);
		concentrations
			.iter()
			.map(|elt_conc| {
				let aliases = elt_conc
					.aliases
					.iter()
					.map(|alias| ElementDose { element: alias.element.clone(), dose: alias.concentration * mult })
					.collect::<Vec<_>>();
				ElementsDosesWithAliases {
					element: elt_conc.element.clone(),
					dose: elt_conc.concentration * mult,
					aliases,
				}
			})
			.sorted()
			.collect::<Vec<_>>()
	}
}

/// A concrete implementation of the dosing by dissolving dry salt in a concentrated solution
#[derive(Debug, Deserialize, Clone, Copy)]
pub struct SolutionDosing {
	container_volume: f64,
	portion_volume: f64,
	dose: f64,
}

impl DiluteMethod for SolutionDosing {
	fn new_from_stdin() -> Result<Self> {
		let input: String = Input::new().with_prompt("Container size in ml: ").interact_text()?;
		let container_volume = input.parse::<f64>()?;
		let input: String = Input::new().with_prompt("Portion size in ml: ").interact_text()?;
		let portion_volume = input.parse::<f64>()?;
		let input: String = Input::new().with_prompt("Dose size in grams (e.g. 2.5): ").interact_text()?;
		let dose = input.parse::<f64>()?;
		Ok(Self { container_volume, portion_volume, dose })
	}

	fn new_from_json(json: &str) -> Result<Self> {
		let res: Self = serde_json::from_str(json)?;
		Ok(res)
	}

	fn dilute(
		&self,
		fertilizer: &Box<dyn Fertilizer>,
		known_elements: &KnownElements,
		tank: &Tank,
	) -> Vec<ElementsDosesWithAliases> {
		let mult = (self.dose * 1000.0 / self.container_volume * self.portion_volume) / tank.effective_volume() as f64;
		let concentrations = fertilizer.components_percentage(known_elements);
		concentrations
			.iter()
			.map(|elt_conc| {
				let aliases = elt_conc
					.aliases
					.iter()
					.map(|alias| ElementDose { element: alias.element.clone(), dose: alias.concentration * mult })
					.collect::<Vec<_>>();
				ElementsDosesWithAliases {
					element: elt_conc.element.clone(),
					dose: elt_conc.concentration * mult,
					aliases,
				}
			})
			.sorted()
			.collect::<Vec<_>>()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{assert_delta_eq, compound::Compound, test_utils::*};

	#[test]
	fn test_kno3_dry() {
		let tank = sample_tank();
		let known_elts = load_known_elements();
		let compound: Box<dyn Fertilizer> = Box::new(Compound::new("KNO3", &known_elts).unwrap());
		let dosing = Box::new(DryDosing(1.0));
		let results = dosing.dilute(&compound, &known_elts, &tank);
		assert!(!results.is_empty());
		assert_eq!(results[0].element.as_str(), "K");
		assert_delta_eq!(results[0].dose, 2.275, MOLAR_MASS_EPSILON);
		assert_eq!(results[1].element.as_str(), "N");
		assert_delta_eq!(results[1].dose, 0.815, MOLAR_MASS_EPSILON);
	}

	#[test]
	fn test_kno3_solution() {
		let tank = sample_tank();
		let known_elts = load_known_elements();
		let compound: Box<dyn Fertilizer> = Box::new(Compound::new("KNO3", &known_elts).unwrap());
		let dosing = Box::new(SolutionDosing { dose: 10.0, container_volume: 1000.0, portion_volume: 100.0 });
		let results = dosing.dilute(&compound, &known_elts, &tank);
		assert!(!results.is_empty());
		assert_eq!(results[0].element.as_str(), "K");
		assert_delta_eq!(results[0].dose, 2.275, MOLAR_MASS_EPSILON);
		assert_eq!(results[1].element.as_str(), "N");
		assert_delta_eq!(results[1].dose, 0.815, MOLAR_MASS_EPSILON);
	}
}
