use crate::traits::Editor;
use anyhow::{anyhow, Result};
use either::Either;
use length::{Length, MetricUnit::*};
use rustyline::Helper;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Formatter};

/// More or less real approximation of the volume to real volume relation
const REAL_VOLUME_MULT: f64 = 0.85;

#[derive(Debug, Deserialize, Serialize, Clone)]
struct LinearDimensions {
	height: f64,
	length: f64,
	width: f64,
}

impl LinearDimensions {
	fn volume(&self) -> f64 {
		self.height * self.length * self.width
	}
}

/// Tank volume holder
#[derive(Deserialize, Serialize, Clone)]
pub struct Tank {
	#[serde(with = "either::serde_untagged")]
	volume: Either<f64, LinearDimensions>,
	#[serde(default)]
	absolute: bool,
}

impl Tank {
	fn length_from_string_as_dm(s: &str) -> Result<f64> {
		let s = s.trim();
		let last_char = s.chars().last().ok_or_else(|| anyhow!("empty dimension"))?;
		if last_char.is_ascii_digit() || last_char == '.' {
			// We assume centimeters and convert them to decimeters to get liters after multiplication
			let dim = s.parse::<f64>()? / 10.0;
			Ok(dim)
		} else {
			let dim = Length::new_string(s)
				.ok_or_else(|| anyhow!("invalid dimension: {}", s))?
				.to(Decimeter);
			Ok(dim.value)
		}
	}
	/// Interactively fill tank dimensions
	pub fn new_from_stdin_linear<T: Helper>(absolute: bool, editor: &mut Editor<T>) -> Result<Self> {
		let input: String = editor.readline("Tank length (e.g. 90cm): ")?;
		let length = Tank::length_from_string_as_dm(input.as_str())?;
		let input: String = editor.readline("Tank width (e.g. 90cm): ")?;
		let width = Tank::length_from_string_as_dm(input.as_str())?;
		let input: String = editor.readline("Tank height (e.g. 90cm): ")?;
		let height = Tank::length_from_string_as_dm(input.as_str())?;

		Ok(Self { volume: Either::Right(LinearDimensions { height, length, width }), absolute })
	}

	/// Load tank from
	pub fn new_from_stdin_volume<T: Helper>(absolute: bool, editor: &mut Editor<T>) -> Result<Self> {
		let input: String = editor.readline("Tank volume in liters: ")?;
		let volume = input.parse::<f64>()?;
		Ok(Self { volume: Either::Left(volume), absolute })
	}

	/// Load tank data from toml
	pub fn new_from_toml(input: &str) -> Result<Self> {
		let tank: Tank = toml::from_str(input)?;
		Ok(tank)
	}

	/// Load tank data from JSON (might be useful in future)
	#[allow(dead_code)]
	pub fn new_from_json(input: &str) -> Result<Self> {
		let tank: Tank = serde_json::from_str(input)?;
		Ok(tank)
	}

	/// Returns a real volume of the tank (approximately volume * 0.9)
	pub fn effective_volume(&self) -> usize {
		let mult = if self.absolute { 1.0 } else { REAL_VOLUME_MULT };
		let vol = match self.volume.as_ref() {
			Either::Left(vol) => *vol,
			Either::Right(lin) => lin.volume(),
		};
		(vol * mult) as usize
	}

	pub fn metric_volume(&self) -> usize {
		(match self.volume.as_ref() {
			Either::Left(vol) => *vol,
			Either::Right(lin) => lin.volume(),
		}) as usize
	}
}

impl Debug for Tank {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "Tank: {} liters real, {} liters nominal", self.effective_volume(), self.metric_volume())?;

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn sample_tank_linear() -> &'static str {
		r#"
		[volume]
		  height = 5.0
		  width = 5.0
		  length = 9.0
		"#
	}
	fn sample_tank_volume() -> &'static str {
		r#"
		volume = 200
		"#
	}

	#[test]
	fn test_tanks_toml() {
		let tank = Tank::new_from_toml(sample_tank_linear()).unwrap();
		assert_eq!(tank.metric_volume(), 225);
		assert_eq!(tank.effective_volume(), 191);
		let tank = Tank::new_from_toml(sample_tank_volume()).unwrap();
		assert_eq!(tank.metric_volume(), 200);
		assert_eq!(tank.effective_volume(), 170);
	}

	fn sample_tank_linear_json() -> &'static str {
		r#"
		{"volume": {
		  "height": 5.0,
		  "width": 5.0,
		  "length": 9.0
		 }
		}
		"#
	}
	fn sample_tank_volume_json() -> &'static str {
		r#"
		{"volume": 200}
		"#
	}

	#[test]
	fn test_tanks_json() {
		let tank = Tank::new_from_json(sample_tank_linear_json()).unwrap();
		assert_eq!(tank.metric_volume(), 225);
		assert_eq!(tank.effective_volume(), 191);
		let tank = Tank::new_from_json(sample_tank_volume_json()).unwrap();
		assert_eq!(tank.metric_volume(), 200);
		assert_eq!(tank.effective_volume(), 170);
	}
}
