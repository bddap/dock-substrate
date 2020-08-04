#!/bin/bash

# Assumes `subkey` is installed, install it as described here https://www.substrate.io/kb/integrate/subkey or
# get docker image from https://hub.docker.com/r/parity/subkey

# The script will will exit if any command fails
set -ueo pipefail

LIBP2P_SECRET_KEY_FILENAME='lib-p2p-secret.key'

if [[ $# -lt 2 ]] || [[ $# -gt 3 ]]
then
  echo "Need at least 2 and at most 3 arguments"
  exit 1
fi

if [[ $1 != 'sr25519' ]] && [[ $1 != 'ed25519' ]] && [[ $1 != 'secp256k1' ]]
then
  echo "First argument is the account key type and must be sr25519 or ed25519 or secp256k1"
  exit 1
fi

if [[ $# == 3 ]]
then
  if [[ $3 != 'main' ]] && [[ $3 != 'test' ]]
  then
    echo "Third argument must be main or test"
    exit 1
  else
    network=$3
  fi
else
  network='test'
fi

account_pass=$2

# Generate account for given key type
if [[ $1 == 'sr25519' ]]
then
  generate_output=$(subkey -s -o=json -p $account_pass generate)
fi

if [[ $1 == 'ed25519' ]]
then
  generate_output=$(subkey -e -o=json -p $account_pass generate)
fi

if [[ $1 == 'secp256k1' ]]
then
  generate_output=$(subkey -k -o=json -p $account_pass generate)
fi


# Extract account address, secret key and secret phrase from above output
ss58_regex='\"ss58Address\": \"([A-Z0-9a-z]+)'
if [[ $generate_output =~ $ss58_regex ]]
then
  account=${BASH_REMATCH[1]}
else
  echo "Cannot get ss58 account address. Regex failed."
  exit 1
fi

secret_phrase_regex='\"secretPhrase\": \"([A-Z0-9a-z ]+)'
if [[ $generate_output =~ $secret_phrase_regex ]]
then
  secret_phrase=${BASH_REMATCH[1]}
else
  echo "Cannot get secret phrase. Regex failed."
  exit 1
fi

secret_key_regex='\"secretSeed\": \"(0x[a-f0-9]+)'
if [[ $generate_output =~ $secret_key_regex ]]
then
  secret_key=${BASH_REMATCH[1]}
else
  echo "Cannot get secret key. Regex failed."
  exit 1
fi

# Generate libp2p keypair, will output public key and persist secret key in file
libp2p_public_key=$(subkey generate-node-key $LIBP2P_SECRET_KEY_FILENAME)

# For colored output
GREEN_COLOR='\033[1;32m'
RED_COLOR='\033[0;31m'
NOCOLOR='\033[0m'

echo -e "The account address in ss58 format is ${GREEN_COLOR}$account${NOCOLOR}"
echo -e "The secret phrase for the account is ${RED_COLOR}$secret_phrase${NOCOLOR}"
echo -e "The secret key for the account is ${RED_COLOR}$secret_key${NOCOLOR}"
echo -e "The libp2p public key is ${GREEN_COLOR}$libp2p_public_key${NOCOLOR}"
echo -e "The libp2p secret key is stored in the file ${RED_COLOR}$LIBP2P_SECRET_KEY_FILENAME${NOCOLOR}"