//! Original Zalith JSON persisted separately from the engine's native touch settings.
use super::layout::Layout;
use std::{
    io::Write,
    path::{Path, PathBuf},
};
pub const MAX_BYTES: usize = 4 * 1024 * 1024;
pub fn path(home: &Path) -> PathBuf {
    home.join("controls/zalith.json")
}
pub fn load(home: &Path) -> Result<Layout, String> {
    let bytes = std::fs::read(path(home)).map_err(|e| e.to_string())?;
    parse(&bytes)
}
pub fn parse(bytes: &[u8]) -> Result<Layout, String> {
    if bytes.len() > MAX_BYTES {
        return Err("Controller JSON exceeds 4 MiB".into());
    }
    let text = std::str::from_utf8(bytes).map_err(|_| "Controller JSON must be UTF-8")?;
    Layout::parse(text)
}
/// Validate before replacing. Preserve imported bytes without schema conversion.
pub fn import(home: &Path, bytes: &[u8]) -> Result<(), String> {
    parse(bytes)?;
    let destination = path(home);
    std::fs::create_dir_all(destination.parent().unwrap()).map_err(|e| e.to_string())?;
    let partial = destination.with_extension("partial");
    let mut file = std::fs::File::create(&partial).map_err(|e| e.to_string())?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())?;
    drop(file);
    std::fs::rename(partial, destination).map_err(|e| e.to_string())
}
pub fn save(home: &Path, layout: &Layout) -> Result<(), String> {
    import(home, layout.to_json().as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Home(PathBuf);
    impl Home {
        fn new() -> Self {
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let id = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "horizon-zalith-{}-{}-{id}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Home {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    const JSON: &str = include_str!("default_layout.json");
    #[test]
    fn imported_zalith_bytes_are_preserved_without_conversion() {
        let home = Home::new();
        import(&home.0, JSON.as_bytes()).unwrap();
        assert_eq!(std::fs::read(path(&home.0)).unwrap(), JSON.as_bytes());
        assert_eq!(load(&home.0).unwrap(), Layout::parse(JSON).unwrap());
    }
    #[test]
    fn invalid_import_keeps_the_existing_layout() {
        let home = Home::new();
        import(&home.0, JSON.as_bytes()).unwrap();
        assert!(import(&home.0, b"{not-json}").is_err());
        assert_eq!(std::fs::read(path(&home.0)).unwrap(), JSON.as_bytes());
    }
    #[test]
    fn oversized_and_non_utf8_imports_do_not_create_a_layout() {
        let home = Home::new();
        assert!(import(&home.0, &vec![b' '; MAX_BYTES + 1]).is_err());
        assert!(import(&home.0, &[0xff]).is_err());
        assert!(!path(&home.0).exists());
    }
    #[test]
    fn editor_save_roundtrips_all_layout_data() {
        let home = Home::new();
        let mut layout = Layout::parse(JSON).unwrap();
        layout.info.name.default = "Custom controller".into();
        save(&home.0, &layout).unwrap();
        assert_eq!(load(&home.0).unwrap(), layout);
    }
}
