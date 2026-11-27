//! Example Argos module: no rights at all, it only computes an answer.
#![forbid(unsafe_code)]

/// The text this module would greet with.
pub const GREETING: &str = "Hello from a small Argos module";

/// Entry logic: the number of words in the greeting, times 7.
pub fn answer() -> i32 {
    let words = GREETING.split_whitespace().count();
    i32::try_from(words).map_or(-1, |w| w * 7)
}

argos_module_sdk::entry!(answer);

#[cfg(test)]
mod tests {
    use super::answer;

    #[test]
    fn the_answer_is_forty_two() {
        assert_eq!(answer(), 42);
    }
}
