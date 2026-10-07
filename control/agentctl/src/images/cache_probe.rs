//! Read-only availability checks for the pinned local image cache.
//!
//! The SDK's image handle comes from its database. Sandbox creation also needs
//! reference-keyed metadata and materialized rootfs artifacts. Do not construct
//! `GlobalCache` here: its constructor creates directories, including for a
//! `workload build --check` invocation.

use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

use microsandbox_image::{
    CachedImageMetadata, Digest, Reference,
    erofs::{ErofsEntryKind, ErofsReader},
};
use sha2::{Digest as _, Sha256};

use super::skew::StoreTag;

fn metadata_path(cache_dir: &Path, reference: &Reference) -> PathBuf {
    let key = Sha256::digest(reference.to_string().as_bytes());
    cache_dir.join("manifests").join(format!("{key:x}.json"))
}

fn regular_file(path: &Path) -> io::Result<Option<std::fs::Metadata>> {
    match std::fs::metadata(path) {
        Ok(meta) if meta.is_file() => Ok(Some(meta)),
        Ok(_) => Ok(None),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn valid_erofs(path: &Path) -> io::Result<bool> {
    let Some(meta) = regular_file(path)? else {
        return Ok(false);
    };
    if meta.len() == 0 || meta.len() % 4096 != 0 {
        return Ok(false);
    }
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    match ErofsReader::new(file).and_then(|mut reader| reader.entry_info("/")) {
        Ok(root) => Ok(root.kind == ErofsEntryKind::Directory),
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::InvalidData
                    | io::ErrorKind::InvalidInput
                    | io::ErrorKind::UnexpectedEof
            ) =>
        {
            Ok(false)
        }
        Err(error) => Err(error),
    }
}

/// Missing or corrupt cache artifacts need reimport; unreadable storage is an
/// error. Match the pinned SDK's layered-cache requirements without modifying
/// the cache, pulling from a registry, or materializing a filesystem.
pub(super) fn cached_tag_state(
    cache_dir: &Path,
    reference: &Reference,
    expected_manifest: Option<&str>,
) -> io::Result<StoreTag> {
    let data = match std::fs::read(metadata_path(cache_dir, reference)) {
        Ok(data) => data,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(StoreTag::Gone),
        Err(error) => return Err(error),
    };
    let Ok(metadata) = serde_json::from_slice::<CachedImageMetadata>(&data) else {
        return Ok(StoreTag::Gone);
    };
    if expected_manifest != Some(metadata.manifest_digest.as_str()) {
        return Ok(StoreTag::Gone);
    }
    let Ok(manifest) = metadata.manifest_digest.parse::<Digest>() else {
        return Ok(StoreTag::Gone);
    };
    for layer in metadata.layers {
        let Ok(diff_id) = layer.diff_id.parse::<Digest>() else {
            return Ok(StoreTag::Gone);
        };
        let path = cache_dir
            .join("layers")
            .join(format!("{}.erofs", diff_id.to_path_safe()));
        if !valid_erofs(&path)? {
            return Ok(StoreTag::Gone);
        }
    }
    let fsmeta = cache_dir
        .join("fsmeta")
        .join(format!("{}.erofs", manifest.to_path_safe()));
    let vmdk = cache_dir
        .join("vmdk")
        .join(format!("{}.vmdk", manifest.to_path_safe()));
    if !valid_erofs(&fsmeta)? || regular_file(&vmdk)?.is_none() {
        return Ok(StoreTag::Gone);
    }
    Ok(StoreTag::Present)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use microsandbox_image::{CachedLayerMetadata, GlobalCache, ImageConfig, erofs::write_erofs};

    const MANIFEST: &str = "sha256:abcdef";
    const LAYER: &str = "sha256:123456";

    fn fixture(cache: &Path, reference: &Reference) -> Result<()> {
        let sdk_cache = GlobalCache::new(cache)?;
        assert_eq!(
            metadata_path(cache, reference),
            sdk_cache.image_metadata_path(reference)
        );
        let metadata = CachedImageMetadata {
            manifest_digest: MANIFEST.into(),
            config_digest: "sha256:fedcba".into(),
            raw_manifest_json: "{}".into(),
            raw_config_json: "{}".into(),
            config: ImageConfig::default(),
            layers: vec![CachedLayerMetadata {
                digest: "sha256:654321".into(),
                media_type: None,
                size_bytes: None,
                diff_id: LAYER.into(),
            }],
        };
        std::fs::write(
            metadata_path(cache, reference),
            serde_json::to_vec(&metadata)?,
        )?;
        let tree = microsandbox_image::tree::FileTree::new();
        write_erofs(&tree, &sdk_cache.layer_erofs_path(&LAYER.parse()?))?;
        write_erofs(&tree, &sdk_cache.fsmeta_erofs_path(&MANIFEST.parse()?))?;
        std::fs::write(sdk_cache.vmdk_path(&MANIFEST.parse()?), b"descriptor")?;
        Ok(())
    }

    #[test]
    fn absent_cache_is_not_created_by_probe() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let absent = temp.path().join("absent");
        let reference = "workestrate-litellm:local".parse()?;
        assert_eq!(
            cached_tag_state(&absent, &reference, Some(MANIFEST))?,
            StoreTag::Gone
        );
        assert!(!absent.exists());
        Ok(())
    }

    #[test]
    fn complete_cache_requires_matching_database_manifest() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let reference = "workestrate-litellm:local".parse()?;
        fixture(temp.path(), &reference)?;
        assert_eq!(
            cached_tag_state(temp.path(), &reference, Some(MANIFEST))?,
            StoreTag::Present
        );
        for expected in [None, Some("sha256:other")] {
            assert_eq!(
                cached_tag_state(temp.path(), &reference, expected)?,
                StoreTag::Gone
            );
        }
        Ok(())
    }

    #[test]
    fn missing_or_corrupt_manifest_needs_import_even_with_all_artifacts() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let reference = "workestrate-litellm:local".parse()?;
        fixture(temp.path(), &reference)?;
        let path = metadata_path(temp.path(), &reference);
        std::fs::remove_file(&path)?;
        assert_eq!(
            cached_tag_state(temp.path(), &reference, Some(MANIFEST))?,
            StoreTag::Gone
        );
        std::fs::write(path, b"not JSON")?;
        assert_eq!(
            cached_tag_state(temp.path(), &reference, Some(MANIFEST))?,
            StoreTag::Gone
        );
        Ok(())
    }

    #[test]
    fn incomplete_rootfs_needs_import() -> Result<()> {
        for relative in [
            "layers/sha256_123456.erofs",
            "fsmeta/sha256_abcdef.erofs",
            "vmdk/sha256_abcdef.vmdk",
        ] {
            let temp = tempfile::tempdir()?;
            let reference = "workestrate-litellm:local".parse()?;
            fixture(temp.path(), &reference)?;
            std::fs::remove_file(temp.path().join(relative))?;
            assert_eq!(
                cached_tag_state(temp.path(), &reference, Some(MANIFEST))?,
                StoreTag::Gone,
                "{relative}"
            );
        }
        Ok(())
    }

    #[test]
    fn aligned_garbage_is_not_a_complete_layer() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let reference = "workestrate-litellm:local".parse()?;
        fixture(temp.path(), &reference)?;
        std::fs::write(temp.path().join("layers/sha256_123456.erofs"), [0; 4096])?;
        assert_eq!(
            cached_tag_state(temp.path(), &reference, Some(MANIFEST))?,
            StoreTag::Gone
        );
        Ok(())
    }

    #[test]
    fn metadata_io_failure_is_not_reported_as_absent() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let reference = "workestrate-litellm:local".parse()?;
        let path = metadata_path(temp.path(), &reference);
        std::fs::create_dir_all(path)?;
        assert!(cached_tag_state(temp.path(), &reference, Some(MANIFEST)).is_err());
        Ok(())
    }
}
