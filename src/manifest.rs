use std::{
    fs::File,
    path::{Path, PathBuf},
};

use types::{errors::Result, extensions::ExtensionManifest};

pub(crate) fn validate_manifest(manifest_path: &Path) -> Result<()> {
    if !manifest_path.exists() {
        return Err(format!("Manifest does not exist at path: {:?}", manifest_path).into());
    }

    let manifest = serde_json::from_reader::<_, ExtensionManifest>(File::open(manifest_path)?);
    match manifest {
        Ok(manifest) => {
            if !manifest.moosync_extension {
                return Err("Manifest is not of a moosync extension".into());
            }

            if !manifest.extension_entry.exists() {
                return Err("Extension path defined in manifest does not exist".into());
            }
        }
        Err(e) => return Err(format!("Failed to validate manifest: {}", e.to_string()).into()),
    }

    Ok(())
}
