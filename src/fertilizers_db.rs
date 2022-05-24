use crate::{compound::Compound, elements::KnownElements, mix::MixedFertilizer, Fertilizer};
use anyhow::{anyhow, Result};
use std::collections::HashMap;

/// All known fertilizers indexed by their name
#[derive(Default)]
pub struct FertilizersDb {
	pub known_fertilizers: HashMap<String, Box<dyn Fertilizer>>,
}

impl FertilizersDb {
	pub fn load_db(&mut self, input: &str, known_elts: &KnownElements) -> Result<()> {
		let res: toml::Value = toml::from_str(input)?;

		if !res.is_table() {
			return Err(anyhow!("known fertilizers must be an object"))
		}

		for (name, obj) in res.as_table().unwrap().iter() {
			if !obj.is_table() {
				return Err(anyhow!("fertilizer {} is not an object", name))
			}

			let fert_obj = obj.as_table().unwrap();

			if fert_obj.contains_key("compounds") {
				let mix = Box::new(MixedFertilizer::new_from_toml_object(name.as_str(), obj, known_elts, true)?);
				self.known_fertilizers.insert(name.clone(), mix as Box<dyn Fertilizer>);
			} else if fert_obj.contains_key("formula") {
				let formula = fert_obj
					.get("formula")
					.unwrap()
					.as_str()
					.ok_or_else(|| anyhow!("formula must be string in {}", name))?;
				let compound = Box::new(Compound::new(formula, known_elts)?);
				self.known_fertilizers.insert(name.clone(), compound as Box<dyn Fertilizer>);
			}
		}

		Ok(())
	}
}
