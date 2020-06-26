//! Generic immutable single-owner storage.

use crate as dock;
use crate::did;
use alloc::vec::Vec;
use codec::{Decode, Encode};
use frame_support::{
    decl_error, decl_module, decl_storage, dispatch::DispatchResult, ensure, traits::Get,
    weights::Weight,
};
use system::ensure_signed;

/// Size of the blob id in bytes
pub const ID_BYTE_SIZE: usize = 32;

/// The unique name for a blob.
pub type BlobId = [u8; ID_BYTE_SIZE];

/// When a new blob is being registered, the following object is sent.
#[derive(Encode, Decode, Clone, PartialEq, Debug)]
pub struct Blob {
    id: BlobId,
    blob: Vec<u8>,
    author: did::Did,
}

pub trait Trait: system::Trait + did::Trait {
    /// Blobs larger than this will not be accepted.
    type MaxBlobSize: Get<u32>;
}

decl_error! {
    /// Error for the token module.
    pub enum BlobError for Module<T: Trait> {
        /// The blob is greater than `MaxBlobSize`
        BlobTooBig,
        /// There is already a blob with same id
        BlobAlreadyExists,
        /// There is no such DID registered
        DidDoesNotExist,
        /// Signature verification failed while adding blob
        InvalidSig
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Blob {
        Blobs get(fn get_blob): map hasher(blake2_128_concat)
            dock::blob::BlobId => Option<(dock::did::Did, Vec<u8>)>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        /// Create a new immutable blob.
        // TODO: Use correct weight offset by benchmarking and consider size of `blob.blob`
        #[weight = T::DbWeight::get().reads_writes(1, 1)  + 0]
        pub fn new(
            origin,
            blob: dock::blob::Blob,
            signature: dock::did::DidSignature,
        ) -> DispatchResult {
            Module::<T>::new_(origin, blob, signature)
        }
    }
}

impl<T: Trait> Module<T> {
    fn new_(
        origin: <T as system::Trait>::Origin,
        blob: Blob,
        signature: did::DidSignature,
    ) -> DispatchResult {
        ensure_signed(origin)?;

        // check
        ensure!(
            T::MaxBlobSize::get() as usize >= blob.blob.len(),
            BlobError::<T>::BlobTooBig
        );
        ensure!(
            !Blobs::contains_key(&blob.id),
            BlobError::<T>::BlobAlreadyExists
        );
        let payload = crate::StateChange::Blob(blob.clone()).encode();
        let valid = did::Module::<T>::verify_sig_from_did(&signature, &payload, &blob.author)?;
        ensure!(valid, BlobError::<T>::InvalidSig);

        // execute
        Blobs::insert(blob.id, (blob.author, blob.blob));

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_common::*;
    pub type BlobMod = crate::blob::Module<Test>;

    /// create a random byte array with set len
    fn random_bytes(len: usize) -> Vec<u8> {
        let ret: Vec<u8> = (0..len).map(|_| rand::random()).collect();
        assert_eq!(ret.len(), len);
        ret
    }

    fn create_blob(
        id: BlobId,
        content: Vec<u8>,
        author: did::Did,
        author_kp: sr25519::Pair,
    ) -> DispatchResult {
        let bl = Blob {
            id,
            blob: content,
            author,
        };
        let sig = sign(&crate::StateChange::Blob(bl.clone()), &author_kp);
        BlobMod::new(Origin::signed(ABBA), bl.clone(), sig)
    }

    fn get_max_blob_size() -> usize {
        <Test as crate::blob::Trait>::MaxBlobSize::get() as usize
    }

    #[test]
    fn add_blob() {
        fn add(size: usize) {
            let id: BlobId = rand::random();
            let noise = random_bytes(size);
            let (author, author_kp) = newdid();
            assert_eq!(Blobs::get(id), None);
            create_blob(id, noise.clone(), author, author_kp).unwrap();
            // Can retrieve a valid blob and the blob contents and author match the given ones.
            assert_eq!(Blobs::get(id), Some((author, noise)));
        }

        ext().execute_with(|| {
            // Can add a blob with unique id, blob data of < MaxBlobSize bytes and a valid signature.
            add(get_max_blob_size() - 1);
            add(get_max_blob_size() - 2);
            add(0);
            // Can add a blob with unique id, blob data of MaxBlobSize bytes and a valid signature.
            add(get_max_blob_size());
        });
    }

    #[test]
    fn err_blob_too_big() {
        fn add_too_big(size: usize) {
            let (author, author_kp) = newdid();
            let noise = random_bytes(size);
            let id = rand::random();
            assert_eq!(Blobs::get(id), None);
            let err = create_blob(id, noise, author, author_kp).unwrap_err();
            assert_eq!(err, BlobError::<Test>::BlobTooBig.into());
            assert_eq!(Blobs::get(id), None);
        }

        ext().execute_with(|| {
            add_too_big(get_max_blob_size() + 1);
            add_too_big(get_max_blob_size() + 2);
        });
    }

    #[test]
    fn err_blob_already_exists() {
        ext().execute_with(|| {
            // Adding a blob with already used id fails with error BlobAlreadyExists.
            let id = rand::random();
            let (author, author_kp) = newdid();
            assert_eq!(Blobs::get(id), None);
            create_blob(id, random_bytes(10), author, author_kp.clone()).unwrap();
            let err = create_blob(id, random_bytes(10), author, author_kp).unwrap_err();
            assert_eq!(err, BlobError::<Test>::BlobAlreadyExists.into());
        });
    }

    #[test]
    fn err_did_does_not_exist() {
        ext().execute_with(|| {
            // Adding a blob with an unregistered DID fails with error DidDoesNotExist.
            let author = rand::random();
            let author_kp = gen_kp();
            let err = create_blob(rand::random(), random_bytes(10), author, author_kp).unwrap_err();
            assert_eq!(err, BlobError::<Test>::DidDoesNotExist.into());
        });
    }

    #[test]
    fn err_invalid_sig() {
        ext().execute_with(|| {
            {
                // An invalid signature while adding a blob should fail with error InvalidSig.
                let (author, author_kp) = newdid();
                let bl = Blob {
                    id: rand::random(),
                    blob: random_bytes(10),
                    author,
                };
                let remreg = crate::revoke::RemoveRegistry {
                    registry_id: rand::random(),
                    last_modified: 10,
                };
                let sig = sign(&crate::StateChange::RemoveRegistry(remreg), &author_kp);
                let err = BlobMod::new(Origin::signed(ABBA), bl.clone(), sig).unwrap_err();
                assert_eq!(err, BlobError::<Test>::InvalidSig.into());
            }

            {
                // signature by other party
                let (author, _) = newdid();
                let (_, author_kp) = newdid();
                let bl = Blob {
                    id: rand::random(),
                    blob: random_bytes(10),
                    author,
                };
                let sig = sign(&crate::StateChange::Blob(bl.clone()), &author_kp);
                let err = BlobMod::new(Origin::signed(ABBA), bl.clone(), sig).unwrap_err();
                assert_eq!(err, BlobError::<Test>::InvalidSig.into());
            }
        })
    }
}
