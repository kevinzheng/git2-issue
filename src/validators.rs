use regex::Regex;
use validator::ValidationError;

lazy_static! {
    pub static ref REGEX_UUID: Regex =
        Regex::new(r"^[a-f0-9]{8}-[a-f0-9]{4}-4[a-f0-9]{3}-[89aAbB][a-f0-9]{3}-[a-f0-9]{12}$")
            .unwrap();
    pub static ref REGEX_GIT_OBJECT_ID: Regex = Regex::new(r"^[a-z0-9]{40}$").unwrap();
}

pub fn validate_git_reference_name(branch_name: &str) -> Result<(), ValidationError> {
    if !git2::Reference::is_valid_name(format!("refs/headers/{}", branch_name).as_str()) {
        // the value of the branch_name will automatically be added later
        return Err(ValidationError::new("invalid_git_reference_name"));
    }
    Ok(())
}
