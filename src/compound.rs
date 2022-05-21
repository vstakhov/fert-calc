use crate::elements::*;
use accurate::{sum::Sum2, traits::*};
use anyhow::{anyhow, Result};
use crossterm::style::Stylize;
use dialoguer::Input;
use std::{
	collections::HashMap,
	fmt::{Debug, Display, Formatter},
};

#[derive(Debug, Default)]
pub struct Compound {
	pub elements: HashMap<Element, u32>,
	pub name: String,
}

impl Display for Compound {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", &self.name)
	}
}

impl PartialEq for Compound {
	fn eq(&self, other: &Self) -> bool {
		self.name == other.name
	}
}

impl Eq for Compound {}

impl Compound {
	fn process_acc<'a>(&mut self, acc: &str, cnt: u32, known_elts: &'a KnownElements) -> Result<&'a Element> {
		let maybe_elt = known_elts.elements.get(acc);

		if let Some(elt) = maybe_elt {
			*self.elements.entry(elt.clone()).or_default() += cnt;
			return Ok(elt)
		}

		Err(anyhow!("Unknown element: {}", acc))
	}

	fn process_trail(
		&mut self,
		last_cnt: Option<u32>,
		last_element: Option<&Element>,
		last_subcompound: &Option<Compound>,
	) -> Result<bool> {
		if let Some(cnt) = last_cnt {
			return if let Some(subcompound) = last_subcompound {
				subcompound.elements.iter().for_each(|(elt, sub_cnt)| {
					*self.elements.entry(elt.clone()).or_default() += sub_cnt * cnt;
				});
				Ok(true)
			} else {
				if let Some(last_elt) = last_element {
					*self.elements.entry(last_elt.clone()).or_default() += cnt - 1;
					Ok(true)
				} else {
					Err(anyhow!("digit without element found"))
				}
			}
		} else if let Some(subcompound) = last_subcompound {
			subcompound.elements.iter().for_each(|(elt, sub_cnt)| {
				*self.elements.entry(elt.clone()).or_default() += sub_cnt;
			});
			return Ok(true)
		}

		Ok(false)
	}

	/// Parses formula from a trivial string knowing some elements
	pub fn new(formula: &str, known_elts: &KnownElements) -> Result<Self> {
		let mut acc = String::new();
		let mut new_compound: Self = Default::default();
		let mut last_element: Option<&Element> = Default::default();
		let mut last_subcompound: Option<Compound> = Default::default();
		let mut last_cnt: Option<u32> = Default::default();
		let mut obraces = 0;
		let mut ebraces = 0;
		new_compound.name = formula.to_owned();

		for chr in formula.chars() {
			if obraces > 0 {
				if chr == ')' {
					ebraces += 1;
				} else if chr == '(' {
					obraces += 1;
				}

				if ebraces != obraces {
					acc.push(chr);
				} else {
					// Here, acc has the whole matching sub-compound
					last_subcompound = Some(Compound::new(acc.as_str(), known_elts)?);
					obraces = 0;
					ebraces = 0;
					acc.clear();
				}
			} else {
				if chr.is_alphabetic() {
					if new_compound.process_trail(last_cnt, last_element, &last_subcompound)? {
						last_element = None;
						last_cnt = None;
						last_subcompound = None;
					}

					acc.push(chr);

					let _ = new_compound.process_acc(acc.as_str(), 1, known_elts).and_then(|elt| {
						acc.clear();
						last_element = Some(elt);
						Ok(())
					});
				} else if chr.is_digit(10) {
					let cnt = chr.to_digit(10).unwrap();

					last_cnt = match last_cnt {
						Some(x) => Some(x * 10 + cnt),
						_ => Some(cnt),
					};
				} else if chr == '(' {
					if !new_compound.process_trail(last_cnt, last_element, &last_subcompound)? && !acc.is_empty() {
						new_compound.process_acc(acc.as_str(), 1, known_elts)?;
					}
					acc.clear();
					last_element = None;
					last_cnt = None;
					last_subcompound = None;
					obraces += 1;
				} else {
					// Ignore garbage stuff
				}
			}
		}

		// Process trail
		if !new_compound.process_trail(last_cnt, last_element, &last_subcompound)? && !acc.is_empty() {
			new_compound.process_acc(acc.as_str(), 1, known_elts)?;
		}

		if new_compound.elements.is_empty() {
			return Err(anyhow!("Empty compound"))
		}

		Ok(new_compound)
	}

	/// Returns a compound from stdin
	pub fn from_stdin(known_elts: &KnownElements) -> Result<Self> {
		let input_compound: String = Input::new().with_prompt("Input compound (e.g. KNO3)").interact_text()?;
		Compound::new(input_compound.as_str(), known_elts)
	}

	/// Returns a molar mass for the compound
	pub fn molar_mass(&self) -> f64 {
		self.elements
			.iter()
			.fold(Sum2::zero(), |acc, (elt, cnt)| acc + elt.molar_mass * (*cnt as f64))
			.sum()
	}

	/// Returns percentage for a specific element
	pub fn element_percentage(&self, element: &Element) -> Option<f64> {
		let molar_mass = self.molar_mass();

		match self.elements.get(element) {
			Some(elt_cnt) => Some(element.molar_mass * (*elt_cnt as f64) / molar_mass * 100.0),
			_ => None,
		}
	}

	/// Returns elements percentage for all elements except unimportant
	pub fn components_percentage(&self, known_elts: &KnownElements) -> Vec<ElementPercentageWithAliases> {
		let molar_mass = self.molar_mass();

		self.elements
			.iter()
			.filter(|(element, _)| return !element.is_insignificant())
			.map(|(element, cnt)| {
				let percentage = element.molar_mass * (*cnt as f64) / molar_mass * 100.0;

				let aliases: Vec<ElementPercentage> = element.aliases.as_ref().map_or(Vec::new(), |aliases| {
					aliases
						.iter()
						.map(|alias| {
							let molecule = Compound::new(alias, known_elts).unwrap();
							let this_elt_percentage = molecule.element_percentage(element).unwrap_or(0.0);
							ElementPercentage {
								element: molecule.name.clone(),
								percentage: percentage / this_elt_percentage * 100.0,
							}
						})
						.collect::<Vec<_>>()
				});

				ElementPercentageWithAliases { element: element.name.clone(), percentage, aliases }
			})
			.collect::<Vec<_>>()
	}
}

/// Element name and it's percentage
pub struct ElementPercentage {
	pub element: String,
	pub percentage: f64,
}
/// A special structure used for an element in a compound + all aliases
pub struct ElementPercentageWithAliases {
	pub element: String,
	pub percentage: f64,
	pub aliases: Vec<ElementPercentage>,
}

impl Debug for ElementPercentageWithAliases {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "Element: {} = {:.2}%", self.element.clone().bold(), self.percentage)?;

		for alias in self.aliases.iter() {
			write!(f, " as {}: {:.2}%", alias.element.clone().bold(), alias.percentage)?;
		}

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::path::Path;

	macro_rules! assert_delta_eq {
		($x:expr, $y:expr, $d:expr) => {
			assert!(
				($x >= $y && $x - $y < $d) || ($x < $y && $y - $x < $d),
				"assert_delta_eq!({}, {}); {:?} != {:?}",
				stringify!($x),
				stringify!($y),
				$x,
				$y
			)
		};
	}

	pub const MOLAR_MASS_EPSILON: f64 = 0.001;

	fn load_known_elements() -> KnownElements {
		KnownElements::new_with_db(Path::new("./elements.json")).unwrap()
	}

	#[test]
	fn parse_simple() {
		let known_elements = load_known_elements();
		let kno3 = Compound::new("KNO3", &known_elements);
		assert_delta_eq!(kno3.as_ref().unwrap().molar_mass(), 101.1032, MOLAR_MASS_EPSILON);
		let kh2po4 = Compound::new("KH2PO4", &known_elements);
		assert_delta_eq!(kh2po4.as_ref().unwrap().molar_mass(), 136.084, MOLAR_MASS_EPSILON);
		let k2h100 = Compound::new("K2H100", &known_elements);
		assert_delta_eq!(k2h100.as_ref().unwrap().molar_mass(), 178.977, MOLAR_MASS_EPSILON);
		let k = Compound::new("K", &known_elements);
		assert_delta_eq!(k.as_ref().unwrap().molar_mass(), 39.098, MOLAR_MASS_EPSILON);
	}

	#[test]
	fn parse_invalid() {
		let known_elements = load_known_elements();
		let invalid = Compound::new("2KO", &known_elements);
		assert!(invalid.is_err());
		let invalid = Compound::new("Ololo", &known_elements);
		assert!(invalid.is_err());
		let invalid = Compound::new("(((Ca(((", &known_elements);
		assert!(invalid.is_err());
	}

	#[test]
	fn parse_braced() {
		let known_elements = load_known_elements();
		let cano3 = Compound::new("Ca(NO3)2", &known_elements);
		assert_delta_eq!(cano3.as_ref().unwrap().molar_mass(), 164.086, MOLAR_MASS_EPSILON);
		let cano3 = Compound::new("(Ca)(NO3)2", &known_elements);
		assert_delta_eq!(cano3.as_ref().unwrap().molar_mass(), 164.086, MOLAR_MASS_EPSILON);
		let cano3 = Compound::new("(Ca)1(NO3)2", &known_elements);
		assert_delta_eq!(cano3.as_ref().unwrap().molar_mass(), 164.086, MOLAR_MASS_EPSILON);
		let braces = Compound::new("(((Ca)))", &known_elements);
		assert_delta_eq!(braces.as_ref().unwrap().molar_mass(), 40.078, MOLAR_MASS_EPSILON);
		let braces = Compound::new("(((Ca)))2", &known_elements);
		assert_delta_eq!(braces.as_ref().unwrap().molar_mass(), 40.078 * 2.0, MOLAR_MASS_EPSILON);
	}
}
