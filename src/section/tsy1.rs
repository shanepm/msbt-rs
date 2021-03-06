use crate::traits::CalculatesSize;
use super::Section;

#[derive(Debug, Clone)]
pub struct Tsy1 {
  pub(crate) section: Section,
  pub(crate) _unknown: Vec<u8>, // tons of unknown data
}

impl Tsy1 {
  pub fn new_unlinked<V: Into<Vec<u8>>>(unknown_bytes: V) -> Self {
    let bytes = unknown_bytes.into();
    Tsy1 {
      section: Section::new(*b"TSY1", bytes.len() as u32),
      _unknown: bytes,
    }
  }

  pub fn section(&self) -> &Section {
    &self.section
  }

  pub fn unknown_bytes(&self) -> &[u8] {
    &self._unknown
  }
}

impl CalculatesSize for Tsy1 {
  fn calc_size(&self) -> usize {
    self.section.calc_size() + self._unknown.len()
  }
}
