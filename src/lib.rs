pub mod analysers;
pub mod cli;
pub mod json;
pub mod output;

const ERR_CONTAINS_UNDERRUN: u8 = 0b0001;
const ERR_CONTAINS_SILENCE: u8 = 0b0010;
