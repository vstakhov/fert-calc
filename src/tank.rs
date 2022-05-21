use anyhow::{anyhow, Result};
use dialoguer::Input;
use length::{Length, MetricUnit::*};
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
}

impl Tank {
	fn length_from_string_as_dm(s: &str) -> Result<f64> {
		let s = s.trim();
		let last_char = s.chars().last().ok_or(anyhow!("empty dimension"))?;
		if last_char.is_digit(10) || last_char == '.' {
			// We assume centimeters and convert them to decimeters to get liters after multiplication
			let dim = s.parse::<f64>()? / 10.0;
			Ok(dim)
		} else {
			let dim = Length::new_string(s).ok_or(anyhow!("invalid dimension: {}", s))?.to(Decimeter);
			Ok(dim.value)
		}
	}
	/// Interactively fill tank dimensions
	pub fn new_from_stdin_linear() -> Result<Self> {
		let input: String = Input::new().with_prompt("Tank length (e.g. 90cm): ").interact_text()?;
		let length = Tank::length_from_string_as_dm(input.as_str())?;
		let input: String = Input::new().with_prompt("Tank width (e.g. 90cm): ").interact_text()?;
		let width = Tank::length_from_string_as_dm(input.as_str())?;
		let input: String = Input::new().with_prompt("Tank height (e.g. 90cm): ").interact_text()?;
		let height = Tank::length_from_string_as_dm(input.as_str())?;

		Ok(Self { linear: Some(LinearDimensions { height, length, width }), volume: Some(length * height * width) })
	}

	/// Load tank from
	pub fn new_from_stdin_volume() -> Result<Self> {
		let input: String = Input::new().with_prompt("Tank volume in liters: ").interact_text()?;
		let volume = input.parse::<f64>()?;
		Ok(Self { linear: None, volume: Some(volume) })
	}

	/// Load tank data from json
	pub fn new_from_json(input: &str) -> Result<Self> {
		let mut tank: Tank = serde_json::from_str(&input)?;
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
		self.volume.map_or(0, |vol| (vol * REAL_VOLUME_MULT) as usize)
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
		r#"{
		"linear": {
			"height": 5.0,
			"width": 5.0,
			"length": 9.0
		}
		}"#
	}
	fn sample_tank_volume() -> &'static str {
		r#"{
		"volume": 200
		}"#
	}

	#[test]
	fn test_tanks() {
		let tank = Tank::new_from_json(sample_tank_linear()).unwrap();
		assert_eq!(tank.metric_volume(), 225);
		assert_eq!(tank.effective_volume(), 191);
		let tank = Tank::new_from_json(sample_tank_volume()).unwrap();
		assert_eq!(tank.metric_volume(), 200);
		assert_eq!(tank.effective_volume(), 170);
	}
}
