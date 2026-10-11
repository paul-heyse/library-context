//! Exact acquired tokenizer assets, loaded once per immutable preparation.
use lctx_model::domain::retrieval::build::invalid;
use lctx_model::domain::{
    ContentHash, ModelError,
    retrieval::partition::{EncodedInput, Tokenizer},
};
use std::{path::Path, sync::Arc};
/// One instance is shared by every documentary grain of the selected realization. Path spelling
/// is not identity; actual acquired assets and effective complete-input processing are.
pub struct LocalTokenizer {
    tokenizer: tokenizers::Tokenizer,
    template: String,
    identity: ContentHash,
}
/// Verification transfers the selected bytes, never a path that can select different bytes later.
struct VerifiedAssets {
    tokenizer: Vec<u8>,
    configuration: Vec<u8>,
}
impl VerifiedAssets {
    fn read(
        root: &Path,
        spec: &lctx_model::domain::embedding::Spec,
    ) -> Result<Self, ModelError> {
        use sha2::{Digest as _, Sha256};
        let manifest = std::fs::read(root.join("SHA256SUMS")).map_err(ModelError::codec)?;
        let revision = format!(
            "sha256:{}",
            ContentHash(Sha256::digest(&manifest).into()).hex()
        );
        if revision != spec.tokenizer_revision {
            return Err(invalid("selected tokenizer manifest differs from encoder"));
        }
        let text = std::str::from_utf8(&manifest).map_err(ModelError::codec)?;
        let mut assets = Vec::new();
        for name in ["tokenizer.json", "tokenizer_config.json", "config.json"] {
            let expected = text
                .lines()
                .find_map(|line| {
                    let (hash, path) = line.split_once(char::is_whitespace)?;
                    (path.trim().trim_start_matches('*') == name).then_some(hash)
                })
                .ok_or_else(|| invalid("tokenizer asset absent from selected manifest"))?;
            let bytes = std::fs::read(root.join(name)).map_err(ModelError::codec)?;
            if ContentHash(Sha256::digest(&bytes).into()).hex() != expected {
                return Err(invalid("acquired tokenizer asset checksum mismatch"));
            }
            assets.push(bytes);
        }
        let mut assets = assets.into_iter();
        Ok(Self {
            tokenizer: assets.next().expect("verified tokenizer"),
            configuration: assets.next().expect("verified tokenizer configuration"),
        })
    }
    fn construct(self, template: &str) -> Result<Arc<LocalTokenizer>, ModelError> {
        LocalTokenizer::from_bytes(&self.tokenizer, &self.configuration, template)
    }
}
impl LocalTokenizer {
    pub fn load_verified(
        root: &Path,
        spec: &lctx_model::domain::embedding::Spec,
    ) -> Result<Arc<Self>, ModelError> {
        VerifiedAssets::read(root, spec)?.construct(&spec.document_template)
    }
    pub fn load(json: &Path, config: &Path, template: &str) -> Result<Arc<Self>, ModelError> {
        let bytes = std::fs::read(json).map_err(ModelError::codec)?;
        let configuration = std::fs::read(config).map_err(ModelError::codec)?;
        Self::from_bytes(&bytes, &configuration, template)
    }
    fn from_bytes(bytes: &[u8], configuration: &[u8], template: &str) -> Result<Arc<Self>, ModelError> {
        if template.matches("{text}").count() != 1 {
            return Err(invalid(
                "tokenizer document template requires exactly one text slot",
            ));
        }
        let settings: serde_json::Value =
            serde_json::from_slice(configuration).map_err(ModelError::codec)?;
        // Tokenizers JSON owns normalization and postprocessing. HF override switches would
        // alter that behavior and require explicit implementation rather than silent divergence.
        for switch in ["add_bos_token", "add_eos_token"] {
            if settings.get(switch).is_some() {
                return Err(invalid(format!(
                    "unsupported tokenizer preprocessing override: {switch}"
                )));
            }
        }
        let mut tokenizer =
            tokenizers::Tokenizer::from_bytes(bytes).map_err(|e| invalid(e.to_string()))?;
        tokenizer
            .with_truncation(None)
            .map_err(|e| invalid(e.to_string()))?;
        tokenizer.with_padding(None);
        let mut identity =
            b"retrieval-tokenizer-v1:byte-offsets,specials=true,truncation=none,padding=none\0"
                .to_vec();
        for asset in [bytes, configuration, template.as_bytes()] {
            identity.extend_from_slice(&(asset.len() as u64).to_le_bytes());
            identity.extend_from_slice(asset);
        }
        Ok(Arc::new(Self {
            tokenizer,
            template: template.into(),
            identity: ContentHash::of(&identity),
        }))
    }
}
impl Tokenizer for LocalTokenizer {
    fn identity(&self) -> ContentHash {
        self.identity
    }
    fn encode(&self, text: &str) -> Result<EncodedInput, ModelError> {
        let (prefix, suffix) = self.template.split_once("{text}").unwrap();
        let input = format!("{prefix}{text}{suffix}");
        let encoded = self
            .tokenizer
            .encode(input.as_str(), true)
            .map_err(|e| invalid(e.to_string()))?;
        Ok(EncodedInput {
            text: input,
            body_start: prefix.len(),
            body_end: prefix.len() + text.len(),
            offsets: encoded.get_offsets().to_vec(),
            specials: encoded
                .get_special_tokens_mask()
                .iter()
                .map(|v| *v != 0)
                .collect(),
        })
    }
}

#[cfg(test)]
mod controls {
    use super::*;
    use tokenizers::{
        TruncationParams, models::wordlevel::WordLevel,
        pre_tokenizers::whitespace::WhitespaceSplit, processors::template::TemplateProcessing,
    };
    #[test]
    fn replacement_after_verification_cannot_change_selected_bytes_or_identity() {
        use sha2::{Digest as _, Sha256};
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        let model = WordLevel::builder()
            .vocab([("[UNK]".to_owned(), 0)].into_iter().collect())
            .unk_token("[UNK]".to_owned())
            .build()
            .unwrap();
        let mut tokenizer = tokenizers::Tokenizer::new(model);
        tokenizer.save(root.join("tokenizer.json"), false).unwrap();
        std::fs::write(root.join("tokenizer_config.json"), b"{}").unwrap();
        std::fs::write(root.join("config.json"), b"{}").unwrap();
        let mut manifest = String::new();
        for name in ["tokenizer.json", "tokenizer_config.json", "config.json"] {
            let bytes = std::fs::read(root.join(name)).unwrap();
            manifest.push_str(&format!("{}  {name}\n", ContentHash(Sha256::digest(bytes).into()).hex()));
        }
        std::fs::write(root.join("SHA256SUMS"), &manifest).unwrap();
        let mut spec = crate::qwen_spec();
        spec.tokenizer_revision = format!("sha256:{}", ContentHash(Sha256::digest(manifest.as_bytes()).into()).hex());
        let original = LocalTokenizer::load_verified(root, &spec).unwrap();
        let retained = VerifiedAssets::read(root, &spec).unwrap();

        // Replace the actual files after verification, before the production constructor boundary.
        tokenizer.with_pre_tokenizer(Some(WhitespaceSplit));
        tokenizer.save(root.join("replacement.json"), false).unwrap();
        std::fs::rename(root.join("replacement.json"), root.join("tokenizer.json")).unwrap();
        std::fs::write(root.join("replacement-config.json"), b"{\"model_max_length\":8192}").unwrap();
        std::fs::rename(root.join("replacement-config.json"), root.join("tokenizer_config.json")).unwrap();
        let replacement = LocalTokenizer::load(&root.join("tokenizer.json"), &root.join("tokenizer_config.json"), &spec.document_template).unwrap();
        assert_eq!(original.encode("hello world").unwrap().offsets.len(), 1);
        assert_eq!(replacement.encode("hello world").unwrap().offsets.len(), 2);
        assert_ne!(original.identity(), replacement.identity());
        let constructed = retained.construct(&spec.document_template).unwrap();
        assert_eq!(constructed.identity(), original.identity());
        assert_eq!(constructed.encode("hello world").unwrap().offsets, original.encode("hello world").unwrap().offsets);
        assert!(LocalTokenizer::load_verified(root, &spec).is_err(), "a new verification must refuse replaced assets under the old manifest");

        // Validating preprocessing consumes the retained configuration as well.
        std::fs::write(root.join("tokenizer_config.json"), b"{\"add_bos_token\":true}").unwrap();
        assert!(LocalTokenizer::load(&root.join("tokenizer.json"), &root.join("tokenizer_config.json"), &spec.document_template).is_err());
        assert_eq!(constructed.encode("hello world").unwrap().offsets.len(), 1);
        spec.tokenizer_revision = "sha256:wrong-manifest".into();
        assert!(VerifiedAssets::read(root, &spec).is_err());
    }
    #[test]
    fn exact_local_assets_count_complete_input_without_saved_truncation_and_keep_byte_offsets() {
        let directory = tempfile::tempdir().unwrap();
        let json = directory.path().join("tokenizer.json");
        let config = directory.path().join("tokenizer_config.json");
        let model = WordLevel::builder()
            .vocab(
                [
                    ("[UNK]".to_owned(), 0),
                    ("header".to_owned(), 1),
                    ("é🦀".to_owned(), 2),
                    ("[EOS]".to_owned(), 3),
                ]
                .into_iter()
                .collect(),
            )
            .unk_token("[UNK]".to_owned())
            .build()
            .unwrap();
        let mut tokenizer = tokenizers::Tokenizer::new(model);
        tokenizer.with_pre_tokenizer(Some(WhitespaceSplit));
        tokenizer.with_post_processor(Some(
            TemplateProcessing::builder()
                .try_single("$A [EOS]")
                .unwrap()
                .special_tokens(vec![("[EOS]", 3)])
                .build()
                .unwrap(),
        ));
        tokenizer
            .with_truncation(Some(TruncationParams {
                max_length: 2,
                ..Default::default()
            }))
            .unwrap();
        tokenizer.save(&json, false).unwrap();
        std::fs::write(
            &config,
            b"{\"tokenizer_class\":\"PreTrainedTokenizerFast\"}",
        )
        .unwrap();
        let local = LocalTokenizer::load(&json, &config, "header {text}").unwrap();
        let encoded = local.encode("é🦀 é🦀").unwrap();
        assert_eq!(
            encoded.offsets.len(),
            4,
            "header + two body tokens + EOS all count"
        );
        assert_eq!(encoded.offsets[1], (7, 13), "Rust offsets are UTF8 bytes");
        assert!(encoded.specials[3]);
        assert_eq!(encoded.offsets[3], (0, 0));
        let identity = local.identity();
        std::fs::write(
            &config,
            b"{\"tokenizer_class\":\"PreTrainedTokenizerFast\",\"model_max_length\":8192}",
        )
        .unwrap();
        assert_ne!(
            identity,
            LocalTokenizer::load(&json, &config, "header {text}")
                .unwrap()
                .identity(),
            "asset configuration is an identity dependency"
        );
        assert_ne!(
            identity,
            LocalTokenizer::load(&json, &config, "{text}")
                .unwrap()
                .identity(),
            "effective input template is an identity dependency"
        );
    }
}
