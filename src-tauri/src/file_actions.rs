use rfd::FileDialog;

pub fn open_directory_picker() -> Result<String, String> {
    match FileDialog::new().pick_folder() {
        Some(path) => Ok(path.to_string_lossy().to_string()),
        None => Err("No directory selected".to_string()),
    }
}
