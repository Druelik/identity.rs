// Copyright 2020-2021 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use identity_core::crypto::KeyPair;
use identity_iota::did::Document;

//Create a Keypair, create a did document from the keypair, sign & verify the document
#[test]
fn test_document() {
  let keypair: KeyPair = KeyPair::new_ed25519().unwrap();

  let mut document: Document = Document::from_keypair(&keypair).unwrap();
  document.sign(keypair.secret()).unwrap();
  assert_eq!(document.verify().unwrap(), ());
}
