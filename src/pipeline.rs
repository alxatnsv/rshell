#[derive(Debug, PartialEq)]
pub enum PipelineError {
    EmptyStage,
}

pub fn split_pipeline<'a>(tokens: &[&'a str]) -> Result<Vec<Vec<&'a str>>, PipelineError> {
    let stages: Vec<Vec<&str>> = tokens.split(|&t| t == "|").map(|s| s.to_vec()).collect();

    if stages.iter().any(|s| s.is_empty()) {
        return Err(PipelineError::EmptyStage);
    }
    Ok(stages)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_command() {
        assert_eq!(split_pipeline(&["ls", "-l"]), Ok(vec![vec!["ls", "-l"]]));
    }

    #[test]
    fn two_stages() {
        assert_eq!(
            split_pipeline(&["ls", "|", "head", "-n", "1"]),
            Ok(vec![vec!["ls"], vec!["head", "-n", "1"]])
        );
    }

    #[test]
    fn leading_pipe() {
        assert_eq!(split_pipeline(&["|", "ls"]), Err(PipelineError::EmptyStage));
    }

    #[test]
    fn trailing_pipe() {
        assert_eq!(split_pipeline(&["ls", "|"]), Err(PipelineError::EmptyStage));
    }

    #[test]
    fn double_pipe() {
        assert_eq!(split_pipeline(&["a", "|", "|", "b"]), Err(PipelineError::EmptyStage));
    }
}