use dialoguer::Input;
use std::error::Error;

pub fn get_confirmation(prompt: String) -> Result<bool, Box<dyn Error>> {
    let input = Input::new()
        .with_prompt(prompt)
        .validate_with(|c_input: &String| -> Result<(), String> {
            if c_input.len() == 1
                && (c_input == "y" || c_input == "Y" || c_input == "n" || c_input == "N")
            {
                Ok(())
            } else if c_input.len() > 1 {
                Err("Enter only one letter (Y/N)".to_string())
            } else {
                Err("Input not valid".to_string())
            }
        })
        .default("Y/N".into())
        .interact_text()?;

    if input == "y" || input == "Y" {
        Ok(true)
    } else {
        Ok(false)
    }
}
