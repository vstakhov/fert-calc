use anyhow::{anyhow, Result};
use length::{Length, MetricUnit::*};
use rustyline::{Editor, Helper};
use serde::Deserialize;
use std::fmt::{Debug, Formatter};

/// More or less real approximation of the volume to real volume relation
const REAL_VOLUME_MULT: f64 = 0.85;

#[derive(Debug, Deserialize, Clone)]
struct LinearDimensions {
	height: f64,
	length: f64,
	width: f64,
}

/// Tank volume holder
#[derive(Deserialize, Clone)]
pub struct Tank {
	linear: Option<LinearDimensions>,
	volume: Option<f64>,
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

		Ok(Self {
			linear: Some(LinearDimensions { height, length, width }),
			volume: Some(length * height * width),
			absolute,
		})
	}

	/// Load tank from
	pub fn new_from_stdin_volume<T: Helper>(absolute: bool, editor: &mut Editor<T>) -> Result<Self> {
		let input: String = editor.readline("Tank volume in liters: ")?;
		let volume = input.parse::<f64>()?;
		Ok(Self { linear: None, volume: Some(volume), absolute })
	}

	/// Load tank data from toml
	pub fn new_from_toml(input: &str) -> Result<Self> {
		let mut tank: Tank = toml::from_str(input)?;
		if tank.volume.is_none() {
			if let Some(lin) = &tank.linear {
				tank.volume = Some(lin.length * lin.height * lin.width);
			} else {
				return Err(anyhow!("Invalid tank specifications"))
			}
		}
		Ok(tank)
	}

	/// Returns a real volume of the tank (approximately volume * 0.9)
	pub fn effective_volume(&self) -> usize {
		let mult = if self.absolute { 1.0 } else { REAL_VOLUME_MULT };
		self.volume.map_or(0, |vol| (vol * mult) as usize)
	}

	pub fn metric_volume(&self) -> usize {
		self.volume.map_or(0, |vol| vol as usize)
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
		absolute = false
		[linear]
		  height = 5.0
		  width = 5.0
		  length = 9.0
		  "#
	}
	fn sample_tank_volume() -> &'static str {
		r#"
		volume = 200
		absolute = false
		"#
	}

	#[test]
	fn test_tanks() {
		let tank = Tank::new_from_toml(sample_tank_linear()).unwrap();
		assert_eq!(tank.metric_volume(), 225);
		assert_eq!(tank.effective_volume(), 191);
		let tank = Tank::new_from_toml(sample_tank_volume()).unwrap();
		assert_eq!(tank.metric_volume(), 200);
		assert_eq!(tank.effective_volume(), 170);
	}
}
