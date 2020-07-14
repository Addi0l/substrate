// This file is part of Substrate.

// Copyright (C) 2019-2020 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or 
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! subcommand utilities
use std::{io::Read, path::PathBuf};
use sp_core::{
	Pair, hexdisplay::HexDisplay,
	crypto::{Ss58Codec, Ss58AddressFormat},
};
use sp_runtime::{MultiSigner, traits::IdentifyAccount};
use crate::{OutputType, error::{self, Error}};
use serde_json::json;
use sp_core::crypto::{SecretString, ExposeSecret};

/// Public key type for Runtime
pub type PublicFor<P> = <P as sp_core::Pair>::Public;
/// Seed type for Runtime
pub type SeedFor<P> = <P as sp_core::Pair>::Seed;

/// helper method to fetch uri from `Option<String>` either as a file or read from stdin
pub fn read_uri(uri: Option<&String>) -> error::Result<String> {
	let uri = if let Some(uri) = uri {
		let file = PathBuf::from(&uri);
		if file.is_file() {
			std::fs::read_to_string(uri)?
				.trim_end()
				.to_owned()
		} else {
			uri.into()
		}
	} else {
		rpassword::read_password_from_tty(Some("URI: "))?
	};

	Ok(uri)
}

/// print formatted pair from uri
pub fn print_from_uri<Pair>(
	uri: &str,
	password: Option<SecretString>,
	network_override: Ss58AddressFormat,
	output: OutputType,
)
	where
		Pair: sp_core::Pair,
		Pair::Public: Into<MultiSigner>,
{
	let password = password.as_ref().map(|s| s.expose_secret().as_str());
	if let Ok((pair, seed)) = Pair::from_phrase(uri, password.clone()) {
		let public_key = pair.public();

		match output {
			OutputType::Json => {
				let json = json!({
						"secretPhrase": uri,
						"secretSeed": format_seed::<Pair>(seed),
						"publicKey": format_public_key::<Pair>(public_key.clone()),
						"accountId": format_account_id::<Pair>(public_key),
						"ss58Address": pair.public().into().into_account().to_ss58check(),
					});
				println!("{}", serde_json::to_string_pretty(&json).expect("Json pretty print failed"));
			},
			OutputType::Text => {
				println!("Secret phrase `{}` is account:\n  \
						Secret seed:      {}\n  \
						Public key (hex): {}\n  \
						Account ID:       {}\n  \
						SS58 Address:     {}",
						uri,
						format_seed::<Pair>(seed),
						format_public_key::<Pair>(public_key.clone()),
						format_account_id::<Pair>(public_key),
						pair.public().into().into_account().to_ss58check(),
				);
			},
		}
	} else if let Ok((pair, seed)) = Pair::from_string_with_seed(uri, password.clone()) {
		let public_key = pair.public();

		match output {
			OutputType::Json => {
				let json = json!({
						"secretKeyUri": uri,
						"secretSeed": if let Some(seed) = seed { format_seed::<Pair>(seed) } else { "n/a".into() },
						"publicKey": format_public_key::<Pair>(public_key.clone()),
						"accountId": format_account_id::<Pair>(public_key),
						"ss58Address": pair.public().into().into_account().to_ss58check(),
					});
				println!("{}", serde_json::to_string_pretty(&json).expect("Json pretty print failed"));
			},
			OutputType::Text => {
				println!("Secret Key URI `{}` is account:\n  \
						Secret seed:      {}\n  \
						Public key (hex): {}\n  \
						Account ID:       {}\n  \
						SS58 Address:     {}",
						uri,
						if let Some(seed) = seed { format_seed::<Pair>(seed) } else { "n/a".into() },
						format_public_key::<Pair>(public_key.clone()),
						format_account_id::<Pair>(public_key),
						pair.public().into().into_account().to_ss58check(),
				);
			},
		}
	} else if let Ok((public_key, _v)) = Pair::Public::from_string_with_version(uri) {
		let v = network_override;

		match output {
			OutputType::Json => {
				let json = json!({
						"publicKeyUri": uri,
						"networkId": String::from(v),
						"publicKey": format_public_key::<Pair>(public_key.clone()),
						"accountId": format_account_id::<Pair>(public_key.clone()),
						"ss58Address": public_key.to_ss58check_with_version(v),
					});
				println!("{}", serde_json::to_string_pretty(&json).expect("Json pretty print failed"));
			},
			OutputType::Text => {
				println!("Public Key URI `{}` is account:\n  \
						Network ID/version: {}\n  \
						Public key (hex):   {}\n  \
						Account ID:         {}\n  \
						SS58 Address:       {}",
					uri,
					String::from(v),
					format_public_key::<Pair>(public_key.clone()),
					format_account_id::<Pair>(public_key.clone()),
					public_key.to_ss58check_with_version(v),
				);
			},
		}
	} else {
		println!("Invalid phrase/URI given");
	}
}

/// generate a pair from suri
pub fn pair_from_suri<P: Pair>(suri: &str, password: Option<&str>) -> Result<P, Error> {
	let pair = P::from_string(suri, password)
		.map_err(|err| format!("Invalid phrase {:?}", err))?;
	Ok(pair)
}

/// formats seed as hex
pub fn format_seed<P: sp_core::Pair>(seed: SeedFor<P>) -> String {
	format!("0x{}", HexDisplay::from(&seed.as_ref()))
}

/// formats public key as hex
fn format_public_key<P: sp_core::Pair>(public_key: PublicFor<P>) -> String {
	format!("0x{}", HexDisplay::from(&public_key.as_ref()))
}

/// formats public key as accountId as hex
fn format_account_id<P: sp_core::Pair>(public_key: PublicFor<P>) -> String
	where
		PublicFor<P>: Into<MultiSigner>,
{
	format!("0x{}", HexDisplay::from(&public_key.into().into_account().as_ref()))
}

/// helper method for decoding hex
pub fn decode_hex<T: AsRef<[u8]>>(message: T) -> Result<Vec<u8>, Error> {
	let mut message = message.as_ref();
	if message[..2] == [b'0', b'x'] {
		message = &message[2..]
	}
	hex::decode(message)
		.map_err(|e| Error::Other(format!("Invalid hex ({})", e)))
}

/// checks if message is Some, otherwise reads message from stdin and optionally decodes hex
pub fn read_message(msg: Option<&String>, should_decode: bool) -> Result<Vec<u8>, Error> {
	let mut message = vec![];
	match msg {
		Some(m) => {
			message = decode_hex(m)?;
		},
		None => {
			std::io::stdin().lock().read_to_end(&mut message)?;
			if should_decode {
				message = decode_hex(&message)?;
			}
		}
	}
	Ok(message)
}


/// Allows for calling $method with appropriate crypto impl.
#[macro_export]
macro_rules! with_crypto_scheme {
	($scheme:expr, $method:ident($($params:expr),*)) => {
		with_crypto_scheme!($scheme, $method<>($($params),*))
	};
	($scheme:expr, $method:ident<$($generics:ty),*>($($params:expr),*)) => {
		match $scheme {
			$crate::CryptoScheme::Ecdsa => {
				$method::<sp_core::ecdsa::Pair, $($generics),*>($($params),*)
			}
			$crate::CryptoScheme::Sr25519 => {
				$method::<sp_core::sr25519::Pair, $($generics),*>($($params),*)
			}
			$crate::CryptoScheme::Ed25519 => {
				$method::<sp_core::ed25519::Pair, $($generics),*>($($params),*)
			}
		}
	};
}
