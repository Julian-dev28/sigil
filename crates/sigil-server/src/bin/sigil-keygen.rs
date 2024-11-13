//! Generate an Ed25519 issuer signing key for Sigil.
//!
//! Prints the PKCS#8 DER private key as base64 (for `SIGIL_ISSUER_KEY_PKCS8_B64`)
//! along with the resulting `did:key`, so operators can provision a stable
//! issuer identity instead of relying on the ephemeral startup key.

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use sigil_core::KeyPair;

fn main() {
    let key = KeyPair::generate();
    let der = key.to_pkcs8_der();
    let b64 = STANDARD.encode(der);

    println!("# Ed25519 issuer key (keep the private key secret)");
    println!("SIGIL_ISSUER_KEY_PKCS8_B64={b64}");
    println!();
    println!("public key multibase : {}", key.public_key_multibase());
    println!("did:key              : {}", key.did_key());
}
