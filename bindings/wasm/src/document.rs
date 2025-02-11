// Copyright 2020-2021 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use identity::core::decode_b58;
use identity::core::FromJson;
use identity::crypto::merkle_key::MerkleKey;
use identity::crypto::merkle_key::MerkleTag;
use identity::crypto::merkle_key::Sha256;
use identity::crypto::merkle_tree::Proof;
use identity::crypto::PublicKey;
use identity::crypto::SecretKey;
use identity::did::verifiable;
use identity::did::Method as CoreMethod;
use identity::did::MethodScope;
use identity::iota::Document as IotaDocument;
use identity::iota::DocumentDiff;
use identity::iota::Method as IotaMethod;
use wasm_bindgen::prelude::*;

use crate::credential::VerifiableCredential;
use crate::credential::VerifiablePresentation;
use crate::crypto::KeyPair;
use crate::crypto::KeyType;
use crate::did::DID;
use crate::method::Method;
use crate::utils::err;

#[wasm_bindgen(inspectable)]
pub struct NewDocument {
  key: KeyPair,
  doc: Document,
}

#[wasm_bindgen]
impl NewDocument {
  #[wasm_bindgen(getter)]
  pub fn key(&self) -> KeyPair {
    self.key.clone()
  }

  #[wasm_bindgen(getter)]
  pub fn doc(&self) -> Document {
    self.doc.clone()
  }
}

// =============================================================================
// =============================================================================

#[wasm_bindgen(inspectable)]
#[derive(Clone, Debug, PartialEq)]
pub struct Document(pub(crate) IotaDocument);

#[wasm_bindgen]
impl Document {
  /// Creates a new DID Document from the given KeyPair.
  #[wasm_bindgen(constructor)]
  #[allow(clippy::new_ret_no_self)]
  pub fn new(type_: KeyType, tag: Option<String>) -> Result<NewDocument, JsValue> {
    let key: KeyPair = KeyPair::new(type_)?;
    let method: IotaMethod = IotaMethod::from_keypair(&key.0, tag.as_deref()).map_err(err)?;
    let document: IotaDocument = IotaDocument::from_authentication(method).map_err(err)?;

    Ok(NewDocument {
      key,
      doc: Self(document),
    })
  }

  /// Creates a new DID Document from the given KeyPair.
  #[wasm_bindgen(js_name = fromKeyPair)]
  pub fn from_keypair(key: &KeyPair) -> Result<Document, JsValue> {
    IotaDocument::from_keypair(&key.0).map_err(err).map(Self)
  }

  /// Creates a new DID Document from the given verification [`method`][`Method`].
  #[wasm_bindgen(js_name = fromAuthentication)]
  pub fn from_authentication(method: &Method) -> Result<Document, JsValue> {
    IotaDocument::from_authentication(method.0.clone())
      .map_err(err)
      .map(Self)
  }

  // ===========================================================================
  // Properties
  // ===========================================================================

  /// Returns the DID Document `id`.
  #[wasm_bindgen(getter)]
  pub fn id(&self) -> DID {
    DID(self.0.id().clone())
  }

  /// Returns the DID Document `proof` object.
  #[wasm_bindgen(getter)]
  pub fn proof(&self) -> Result<JsValue, JsValue> {
    match self.0.proof() {
      Some(proof) => JsValue::from_serde(proof).map_err(err),
      None => Ok(JsValue::NULL),
    }
  }

  // ===========================================================================
  // Verification Methods
  // ===========================================================================

  #[wasm_bindgen(js_name = insertMethod)]
  pub fn insert_method(&mut self, method: &Method, scope: Option<String>) -> Result<bool, JsValue> {
    let scope: MethodScope = scope.unwrap_or_default().parse().map_err(err)?;

    Ok(self.0.insert_method(scope, method.0.clone()))
  }

  #[wasm_bindgen(js_name = removeMethod)]
  pub fn remove_method(&mut self, did: &DID) -> Result<(), JsValue> {
    self.0.remove_method(&did.0).map_err(err)
  }

  // ===========================================================================
  // Signatures
  // ===========================================================================

  /// Signs the DID Document with the default authentication method.
  #[wasm_bindgen]
  pub fn sign(&mut self, key: &KeyPair) -> Result<(), JsValue> {
    self.0.sign(key.0.secret()).map_err(err)
  }

  /// Verify the signature with the authentication_key
  #[wasm_bindgen]
  pub fn verify(&self) -> bool {
    self.0.verify().is_ok()
  }

  #[wasm_bindgen(js_name = signCredential)]
  pub fn sign_credential(&self, data: &JsValue, args: &JsValue) -> Result<VerifiableCredential, JsValue> {
    let json: JsValue = self.sign_data(data, args)?;
    let data: VerifiableCredential = VerifiableCredential::from_json(&json)?;

    Ok(data)
  }

  #[wasm_bindgen(js_name = signPresentation)]
  pub fn sign_presentation(&self, data: &JsValue, args: &JsValue) -> Result<VerifiablePresentation, JsValue> {
    let json: JsValue = self.sign_data(data, args)?;
    let data: VerifiablePresentation = VerifiablePresentation::from_json(&json)?;

    Ok(data)
  }

  /// Creates a signature for the given `data` with the specified DID Document
  /// Verification Method.
  ///
  /// An additional `proof` property is required if using a Merkle Key
  /// Collection verification Method.
  #[wasm_bindgen(js_name = signData)]
  pub fn sign_data(&self, data: &JsValue, args: &JsValue) -> Result<JsValue, JsValue> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Args {
      MerkleKey {
        method: String,
        public: String,
        secret: String,
        proof: String,
      },
      Default {
        method: String,
        secret: String,
      },
    }

    let mut data: verifiable::Properties = data.into_serde().map_err(err)?;
    let args: Args = args.into_serde().map_err(err)?;

    match args {
      Args::MerkleKey {
        method,
        public,
        secret,
        proof,
      } => {
        let merkle_key: Vec<u8> = self
          .0
          .try_resolve(&*method)
          .and_then(|method| method.key_data().try_decode())
          .map_err(err)?;

        let public: PublicKey = decode_b58(&public).map_err(err).map(Into::into)?;
        let secret: SecretKey = decode_b58(&secret).map_err(err).map(Into::into)?;

        let digest: MerkleTag = MerkleKey::extract_tags(&merkle_key).map_err(err)?.1;
        let proof: Vec<u8> = decode_b58(&proof).map_err(err)?;

        let signer: _ = self.0.signer(&secret).method(&method);

        match digest {
          MerkleTag::SHA256 => match Proof::<Sha256>::decode(&proof) {
            Some(proof) => signer.merkle_key((&public, &proof)).sign(&mut data).map_err(err)?,
            None => return Err("Invalid Public Key Proof".into()),
          },
          _ => return Err("Invalid Merkle Key Digest".into()),
        }
      }
      Args::Default { method, secret } => {
        let secret: SecretKey = decode_b58(&secret).map_err(err).map(Into::into)?;

        self.0.signer(&secret).method(&method).sign(&mut data).map_err(err)?;
      }
    }

    JsValue::from_serde(&data).map_err(err)
  }

  /// Verifies the authenticity of `data` using the target verification method.
  #[wasm_bindgen(js_name = verifyData)]
  pub fn verify_data(&self, data: &JsValue) -> Result<bool, JsValue> {
    let data: verifiable::Properties = data.into_serde().map_err(err)?;
    let result: bool = self.0.verifier().verify(&data).is_ok();

    Ok(result)
  }

  #[wasm_bindgen(js_name = resolveKey)]
  pub fn resolve_key(&mut self, query: &str) -> Result<Method, JsValue> {
    let method: CoreMethod = self.0.try_resolve(query).map_err(err)?.clone();

    IotaMethod::try_from_core(method).map_err(err).map(Method)
  }

  #[wasm_bindgen(js_name = revokeMerkleKey)]
  pub fn revoke_merkle_key(&mut self, query: &str, index: usize) -> Result<bool, JsValue> {
    let method: &mut IotaMethod = self
      .0
      .try_resolve_mut(query)
      .and_then(IotaMethod::try_from_mut)
      .map_err(err)?;

    method.revoke_merkle_key(index).map_err(err)
  }

  // ===========================================================================
  // Diffs
  // ===========================================================================

  /// Generate the difference between two DID Documents and sign it
  #[wasm_bindgen]
  pub fn diff(&self, other: &Document, message: &str, key: &KeyPair) -> Result<JsValue, JsValue> {
    self
      .0
      .diff(&other.0, message.to_string().into(), key.0.secret())
      .map_err(err)
      .and_then(|diff| JsValue::from_serde(&diff).map_err(err))
  }

  /// Verifies the `diff` signature and merges the changes into `self`.
  #[wasm_bindgen]
  pub fn merge(&mut self, diff: &str) -> Result<(), JsValue> {
    let diff: DocumentDiff = DocumentDiff::from_json(diff).map_err(err)?;

    self.0.merge(&diff).map_err(err)?;

    Ok(())
  }

  /// Serializes a `Document` object as a JSON object.
  #[wasm_bindgen(js_name = toJSON)]
  pub fn to_json(&self) -> Result<JsValue, JsValue> {
    JsValue::from_serde(&self.0).map_err(err)
  }

  /// Deserializes a `Document` object from a JSON object.
  #[wasm_bindgen(js_name = fromJSON)]
  pub fn from_json(json: &JsValue) -> Result<Document, JsValue> {
    json.into_serde().map_err(err).map(Self)
  }
}
