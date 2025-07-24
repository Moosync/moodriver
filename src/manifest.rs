use std::{fs::File, path::Path};

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

            let ext_entry = if let Some(parent) = manifest_path.parent() {
                parent.join(manifest.extension_entry)
            } else {
                manifest.extension_entry
            };
            if !ext_entry.exists() {
                return Err(format!("Extension path: {:?} does not exist", ext_entry).into());
            }
        }
        Err(e) => return Err(format!("Failed to validate manifest: {}", e).into()),
    }

    Ok(())
}
