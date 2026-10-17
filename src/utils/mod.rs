pub mod analysis_helpers;
pub mod language_parser;
pub mod option_ext;
pub mod risk_analyzer;
pub mod validation_printer;

pub use analysis_helpers::*;
pub use language_parser::*;
pub use option_ext::*;
pub use risk_analyzer::*;
pub use validation_printer::*;

/// Free a large value off the critical path; its destructor has no observable effects.
pub fn drop_in_background<T: Send + 'static>(value: T) {
    std::thread::spawn(move || drop(value));
}
