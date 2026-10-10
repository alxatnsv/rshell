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
