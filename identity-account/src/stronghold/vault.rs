// Copyright 2020-2021 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use std::path::Path;
use iota_stronghold::StrongholdFlags;
use iota_stronghold::Location;
use iota_stronghold::RecordHint;
use iota_stronghold::VaultFlags;
use iota_stronghold::Procedure;
use iota_stronghold::SLIP10DeriveInput;
use iota_stronghold::hd::Chain;
use iota_stronghold::hd::ChainCode;

use crate::error::Error;
use crate::error::Result;
use crate::error::PleaseDontMakeYourOwnResult;
use crate::stronghold::Runtime;
use crate::stronghold::ProcedureResult;

pub type Record = (usize, RecordHint);

#[derive(Debug)]
pub struct Vault<'path> {
  flags: Vec<StrongholdFlags>,
  name: Vec<u8>,
  path: &'path Path,
}

impl<'path> Vault<'path> {
  pub(crate) fn new<T>(path: &'path Path, name: &T, flags: &[StrongholdFlags]) -> Self
  where
    T: AsRef<[u8]> + ?Sized,
  {
    Self {
      flags: flags.to_vec(),
      name: name.as_ref().to_vec(),
      path,
    }
  }
}

impl Vault<'_> {
  pub fn name(&self) -> &[u8] {
    &self.name
  }

  pub fn path(&self) -> &Path {
    self.path
  }

  pub fn flags(&self) -> &[StrongholdFlags] {
    &self.flags
  }

  pub async fn flush(&self) -> Result<()> {
    Runtime::lock().await?.write_snapshot(self.path).await
  }

  /// Inserts a record.
  pub async fn insert(&self, location: Location, payload: Vec<u8>, hint: RecordHint, flags: &[VaultFlags]) -> Result<()> {
    let mut runtime: _ = Runtime::lock().await?;

    runtime.set_snapshot(self.path).await?;
    runtime.load_actor(self.path, &self.name, &self.flags).await?;
    runtime.write_to_vault(location, payload, hint, flags.to_vec()).await.to_result()?;

    Ok(())
  }

  /// Deletes a record.
  pub async fn delete(&self, location: Location, gc: bool) -> Result<()> {
    let mut runtime: _ = Runtime::lock().await?;

    runtime.set_snapshot(self.path).await?;
    runtime.load_actor(self.path, &self.name, &self.flags).await?;
    runtime.delete_data(location, gc).await.to_result()?;

    Ok(())
  }

  /// Executes a runtime `procedure`.
  pub async fn execute(&self, procedure: Procedure) -> Result<ProcedureResult> {
    let mut runtime: _ = Runtime::lock().await?;

    runtime.set_snapshot(self.path).await?;
    runtime.load_actor(self.path, &self.name, &self.flags).await?;

    runtime.runtime_exec(procedure).await.to_result()
  }

  /// Returns a list of available records and hints.
  pub async fn records<T>(&self, vault: &T) -> Result<Vec<Record>>
  where
    T: AsRef<[u8]> + ?Sized,
  {
    let mut runtime: _ = Runtime::lock().await?;

    runtime.set_snapshot(self.path).await?;
    runtime.load_actor(self.path, &self.name, &self.flags).await?;

    let (data, status): (Vec<Record>, _) = runtime
      .list_hints_and_ids(vault.as_ref())
      .await;

    status.to_result()?;

    Ok(data)
  }

  pub async fn slip10_generate(
    &self,
    output: Location,
    hint: RecordHint,
    bytes: Option<usize>,
  ) -> Result<()> {
    let procedure: Procedure = Procedure::SLIP10Generate {
      output,
      hint,
      size_bytes: bytes,
    };

    match self.execute(procedure).await? {
      ProcedureResult::SLIP10Generate => Ok(()),
      _ => Err(Error::StrongholdProcedureFailure),
    }
  }

  pub async fn slip10_derive(
    &self,
    chain: Chain,
    input: SLIP10DeriveInput,
    output: Location,
    hint: RecordHint,
  ) -> Result<ChainCode> {
    let procedure: Procedure = Procedure::SLIP10Derive {
      chain,
      input,
      output,
      hint,
    };

    match self.execute(procedure).await? {
      ProcedureResult::SLIP10Derive(chaincode) => Ok(chaincode),
      _ => Err(Error::StrongholdProcedureFailure),
    }
  }

  pub async fn bip39_recover<P>(
    &self,
    mnemonic: String,
    output: Location,
    passphrase: P,
    hint: RecordHint,
  ) -> Result<()>
  where
    P: Into<Option<String>>,
  {
    let procedure: Procedure = Procedure::BIP39Recover {
      mnemonic,
      passphrase: passphrase.into(),
      output,
      hint,
    };

    match self.execute(procedure).await? {
      ProcedureResult::BIP39Recover => Ok(()),
      _ => Err(Error::StrongholdProcedureFailure),
    }
  }

  pub async fn bip39_generate<P>(
    &self,
    output: Location,
    passphrase: P,
    hint: RecordHint,
  ) -> Result<()>
  where
    P: Into<Option<String>>,
  {
    let procedure: Procedure = Procedure::BIP39Generate {
      passphrase: passphrase.into(),
      output,
      hint,
    };

    match self.execute(procedure).await? {
      ProcedureResult::BIP39Generate => Ok(()),
      _ => Err(Error::StrongholdProcedureFailure),
    }
  }

  pub async fn bip39_mnemonic_sentence(&self, seed: Location) -> Result<String> {
    let procedure: Procedure = Procedure::BIP39MnemonicSentence { seed };

    match self.execute(procedure).await? {
      ProcedureResult::BIP39MnemonicSentence(mnemonic) => Ok(mnemonic),
      _ => Err(Error::StrongholdProcedureFailure),
    }
  }

  pub async fn ed25519_public_key(&self, private_key: Location) -> Result<[u8; 32]> {
    let procedure: Procedure = Procedure::Ed25519PublicKey { private_key };

    match self.execute(procedure).await? {
      ProcedureResult::Ed25519PublicKey(public_key) => Ok(public_key),
      _ => Err(Error::StrongholdProcedureFailure),
    }
  }

  pub async fn ed25519_sign(&self, msg: Vec<u8>, private_key: Location) -> Result<[u8; 64]> {
    let procedure: Procedure = Procedure::Ed25519Sign { private_key, msg };

    match self.execute(procedure).await? {
      ProcedureResult::Ed25519Sign(signature) => Ok(signature),
      _ => Err(Error::StrongholdProcedureFailure),
    }
  }
}
