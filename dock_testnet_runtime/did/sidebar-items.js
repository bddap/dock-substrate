initSidebarItems({"enum":[["Call","Dispatchable calls."],["DidSignature","An abstraction for a signature."],["Error","Error for the token module."],["Event","Events for this module."],["PublicKey","An abstraction for a public key. Abstracts the type and value of the public key where the value is a byte array"]],"struct":[["Bytes32","A wrapper over 32-byte array"],["Bytes33","A wrapper over a byte array"],["Bytes64","A wrapper over a byte array"],["Bytes65","A wrapper over a byte array"],["DidRemoval","This struct is passed as an argument while removing the DID `did` is the DID which is being removed. `last_modified_in_block` is the block number when this DID was last modified. The last modified time is present to prevent replay attack."],["KeyDetail","`controller` is the controller DID and its value might be same as `did`. `public_key` is the public key and it is accepted and stored as raw bytes."],["KeyUpdate","This struct is passed as an argument while updating the key for a DID. `did` is the DID whose key is being updated. `public_key` the new public key `controller` If provided None, the controller is unchanged. While serializing, use literal \"None\" when controller is None. The last_modified_in_block is the block number when this DID was last modified. It is used to prevent replay attacks. This approach allows easy submission of 1 update transaction in a block. It's theoretically possible to submit more than one txn per block, but the method is non-trivial and potentially unreliable. An alternate approach can be to have a nonce associated to each detail which is incremented on each successful extrinsic and the chain requiring the extrinsic's nonce to be higher than current. This is little more involved as it involves a \">\" check"],["Module",""]],"trait":[["BigArray",""],["Trait","The module's configuration trait."]],"type":[["Did","The type of the Dock DID"]]});